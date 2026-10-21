# Phân Tích Nghiệp Vụ & Kinh Tế Học Prompt Caching (Business Analysis)

- **Tài liệu:** Phân tích nghiệp vụ hệ thống `droid-cache-proxy` (`dcp`)
- **Mục tiêu:** Làm rõ cơ chế kỹ thuật của Prompt Caching, giải phẫu request của coding agent, phân loại nguyên nhân gây cache miss, ma trận rủi ro và phương pháp luận đo lường kinh tế học.

---

## 1. Cơ Chế Hoạt Động Của Prompt Caching

### 1.1. Bản chất kỹ thuật (Prefix-based KV Cache Reuse)
Trong mô hình Transformer tự hồi quy (Autoregressive LLM), chi phí tính toán lớn nhất trong pha xử lý prompt (prefill phase) là việc tính toán ma trận Key-Value (KV Cache) cho toàn bộ chuỗi token đầu vào.

Cơ chế Prompt Caching của các provider hiện nay (OpenAI, DeepSeek, Anthropic, vLLM, SGLang) hoạt động dựa trên nguyên lý **Trùng khớp tiền tố (Exact Prefix Matching)**:
- Provider duy trì KV Cache trên bộ nhớ GPU/vRAM hoặc host RAM của cluster máy chủ.
- Khi một request mới đến, provider sẽ so sánh chuỗi token từ token đầu tiên ($T_0$) cho đến token thứ $k$ ($T_k$).
- Nếu $T_0 \dots T_k$ khớp chính xác từng token với một đoạn KV Cache đã có sẵn trong bộ nhớ:
  - Provider **bỏ qua việc tính toán prefill** cho $k$ token đó, nạp thẳng KV cache có sẵn vào bộ nhớ.
  - Quá trình prefill chỉ thực hiện từ token thứ $k+1$ trở đi.
- **Quy tắc gãy tiền tố (Prefix Break Invariant):** Nếu có bất kỳ sự khác biệt nào ở token thứ $i$ (dù chỉ là 1 dấu cách, 1 ký tự viết hoa/thường hay 1 giây nhảy số trên timestamp), **toàn bộ các token từ vị trí $i$ trở về sau đều bị cache miss**, ngay cả khi phần nội dung phía sau đó dài hàng chục nghìn token và giống hệt như cũ.

```text
Request N:   [System: Rules + Tools] [User: Read file A] [Tool: Content A] [User: Fix bug]
             |------- 30,000 tokens -------|
             ===============================> 100% KV Cache Reused

Request N+1: [System: Rules (Timestamp +1s) + Tools] [User: Read file A] ...
             |-- 50 tokens --| (GÃY TẠI ĐÂY)
             X (Miss hoàn toàn 29,950 tokens phía sau vì prefix bị lệch)
```

### 1.2. Các thông số kỹ thuật then chốt của Provider
1. **Ngưỡng tối thiểu (Minimum Cache Threshold):**
   - **OpenAI:** Tối thiểu 1024 prompt tokens. Các request ngắn hơn 1024 tokens sẽ không được kích hoạt prompt caching.
   - **DeepSeek:** Hoạt động theo block (thường là block 64 tokens).
   - **Anthropic:** Tối thiểu 1024 tokens (Claude 3.5 Sonnet) hoặc 2048 tokens (Claude 3 Opus).
2. **Thời gian tồn tại của Cache (Cache TTL):**
   - Cache được tự động gia hạn (refreshed) mỗi khi có request tái sử dụng.
   - Nếu không có request mới trúng tiền tố đó trong khoảng thời gian từ **5 đến 10 phút**, hệ thống máy chủ sẽ giải phóng KV Cache này để nhường chỗ cho các request khác.
3. **Kinh tế học & Tỷ lệ chiết khấu giá (Pricing Impact):**
   - Giá của Cached Prompt Tokens thường chỉ bằng **10% đến 50%** so với Uncached Prompt Tokens:
     - OpenAI GPT-4o / GPT-4o-mini: Giảm giá 50% cho cached tokens.
     - DeepSeek-V3 / DeepSeek-R1: Giảm giá lên tới 75% - 90% cho cached tokens (Uncached: $0.14/M token, Cached: $0.014/M token).
   - Khi coding agent thực hiện 20 lượt đối thoại trong một phiên làm việc, tổng số prompt tokens tích lũy có thể lên tới 1.000.000 tokens. Tỷ lệ cache hit 80% giúp tiết kiệm tới 40% - 70% tổng chi phí cho phiên đó.
4. **Vị trí đọc số liệu Cache Hit (Usage Reporting):**
   - Trong response JSON chuẩn OpenAI:
     ```json
     "usage": {
       "prompt_tokens": 32450,
       "completion_tokens": 420,
       "total_tokens": 32870,
       "prompt_tokens_details": {
         "cached_tokens": 28160
       }
     }
     ```
   - Trong response chuẩn DeepSeek:
     ```json
     "usage": {
       "prompt_tokens": 32450,
       "completion_tokens": 420,
       "prompt_cache_hit_tokens": 28160,
       "prompt_cache_miss_tokens": 4290
     }
     ```
   - **Điểm lưu ý khi Streaming:** Với các request streaming (`stream: true`), upstream chỉ trả về block `usage` ở chunk cuối cùng nếu request có thiết lập `stream_options: { "include_usage": true }`. Nếu không có cờ này, chunk cuối sẽ chỉ có `choices: []` và không có `usage`.

---

## 2. Giải Phẫu Request Của Coding Agent (Agent Request Anatomy)

Một request gửi từ coding agent đến endpoint `/v1/chat/completions` thường có cấu trúc phân tầng như sau:

```text
┌─────────────────────────────────────────────────────────────────┐
│ 1. SYSTEM PROMPT (Tĩnh hoặc Bán tĩnh)                            │
│    - Role & Operating Guidelines (Quy tắc hành vi, Persona)      │
│    - Workspace Environment (OS, arch, working directory, git)   │
│    - Volatile Injection: Current timestamp, process ID          │
├─────────────────────────────────────────────────────────────────┤
│ 2. TOOLS DEFINITIONS (Tĩnh)                                      │
│    - Array of functions: read_file, write_file, execute_bash... │
│    - JSON Schema for parameters (type, properties, required)    │
├─────────────────────────────────────────────────────────────────┤
│ 3. CONVERSATION HISTORY (Bán tĩnh - Tích lũy theo từng turn)    │
│    - Turn 1: User prompt ban đầu ("Hãy sửa lỗi bug #102")        │
│    - Turn 1: Assistant response (tool_calls: [read_file(path)]) │
│    - Turn 1: Tool message (content: "File content 500 lines...")│
│    - Turn 2: Assistant response (tool_calls: [edit_file(...)])  │
│    - Turn 2: Tool message (content: "Edit applied")             │
├─────────────────────────────────────────────────────────────────┤
│ 4. LATEST TURN / DYNAMIC DATA (Động hoàn toàn)                  │
│    - User instruction mới nhất hoặc kết quả thực thi vừa nhận   │
└─────────────────────────────────────────────────────────────────┘
```

### Phân tích tính biến thiên:
- **Tĩnh tuyệt đối (Invariant):** Các chỉ dẫn quy tắc cốt lõi của Agent, danh mục công cụ hỗ trợ.
- **Bán tĩnh (Semi-static):** Lịch sử các lượt đối thoại trước đó. Chúng chỉ mở rộng ở phần đuôi, không thay đổi phần đầu.
- **Biến thiên liên tục (Volatile):**
  - Thời gian hiện tại tính bằng giây/phút.
  - Trạng thái git status (file modified thay đổi sau mỗi lượt tool).
  - Kết quả tra cứu tài liệu động được chèn xen kẽ.

---

## 3. Hệ Thống Phân Loại Nguyên Nhân Cache Miss (Miss Taxonomy)

Từ nghiên cứu hành vi thực tế của các coding agent, chúng tôi xây dựng bảng phân loại nguyên nhân gây cache miss để bộ phân tích (Analyzer) giai đoạn 2 kiểm tra:

| Mã | Tên nguyên nhân | Cơ chế gây lỗi | Mức độ phổ biến | Biện pháp xử lý (Giai đoạn 2) |
|---|---|---|---|---|
| **M1** | **Volatile System Timestamp** | System prompt chứa chuỗi ngày giờ chi tiết (`Current time: 2026-10-20 10:15:30`). Mỗi turn cách nhau vài chục giây khiến toàn bộ prefix bị lệch ngay từ đầu. | Rất cao | Tách timestamp ra khỏi system prompt, đưa xuống user message cuối cùng hoặc chuẩn hóa làm tròn. |
| **M2** | **Tool Order Shuffling** | Mảng `tools` bị thay đổi thứ tự do cấu trúc dữ liệu Set/Map không có thứ tự trong mã nguồn của agent hoặc plugin nạp không đồng nhất. | Trung bình | Sắp xếp mảng `tools` theo thứ tự alphabet của `function.name`. |
| **M3** | **Non-canonical JSON Schema** | Thứ tự các keys trong `parameters.properties` của tools hoặc khoảng trắng thụt lề JSON khác nhau giữa các lần gửi. | Cao | Canonicalize JSON: sắp xếp alphabet các keys, loại bỏ khoảng trắng dư thừa, chuẩn hóa Unicode NFC. |
| **M4** | **Dynamic Tool Results Xen Kẽ** | Agent chèn nội dung file vừa đọc hoặc kết quả git status lên đầu danh sách tin nhắn thay vì nối vào cuối lịch sử hội thoại. | Trung bình | Căn chỉnh prefix (`prefix_align`): Giữ nguyên thứ tự thời gian của hội thoại, không cho phép chèn message động lên trước system/tools. |
| **M5** | **Cache TTL Expiry** | Lập trình viên dừng lại suy nghĩ, đọc code hoặc đi ra ngoài quá 10 phút, vượt quá thời gian lưu cache trên máy chủ provider. | Tự nhiên | Không can thiệp được bằng transform; ghi nhận trong báo cáo để phân biệt với lỗi kỹ thuật. |
| **M6** | **Sub-threshold Token Count** | Lượt hỏi đáp đầu tiên có độ dài quá ngắn (< 1024 tokens), không đủ điều kiện để provider kích hoạt cache. | Tự nhiên | Ghi nhận trạng thái `sub_threshold` trong metrics. |

---

## 4. Các Trường Hợp Sử Dụng (Use Cases & Acceptance Criteria)

### UC-1: Khởi động và Forward trong suốt (Giai đoạn 1)
- **Tác nhân:** Kỹ sư phần mềm.
- **Mô tả:** Người dùng cấu hình biến môi trường `DCP_API_KEY`, khởi động proxy bằng lệnh `dcp serve`, sau đó trỏ coding agent về `http://127.0.0.1:6789/v1`.
- **Tiêu chí nghiệm thu:**
  1. Proxy bind thành công tại `127.0.0.1:6789`.
  2. Mọi request từ agent (GET `/v1/models`, POST `/v1/chat/completions`) đều được chuyển tiếp tới upstream thành công.
  3. Header `Authorization` luôn chứa API key chuẩn từ config, client không cần cấu hình key thật.
  4. Luồng streaming phản hồi mượt mà, không giật lag, TTFT tăng thêm do proxy không quá 2ms.
  5. Proxy tự động tạo thư mục `~/.dcp/` và file database SQLite nếu chưa tồn tại.

### UC-2: Đo lường và Báo cáo Cache Hit Baseline (Giai đoạn 1)
- **Tác nhân:** Kỹ sư phần mềm / Script kiểm tra.
- **Mô tả:** Sau khi hoàn thành một phiên lập trình, người dùng kiểm tra tỷ lệ cache hit thực tế.
- **Tiêu chí nghiệm thu:**
  1. Lệnh `dcp stats` hiển thị bảng thống kê: Tổng số request, số request stream/non-stream, tổng prompt tokens, tổng cached tokens, tỷ lệ Token Hit Rate (%) và Request Hit Rate (%).
  2. Endpoint `GET /_dcp/metrics` trả về JSON chứa đầy đủ các chỉ số trên.
  3. Dữ liệu khớp với thông tin trả về trong block `usage` của upstream.

### UC-3: Phân tích Log & Phát hiện Điểm Gãy Prefix (Giai đoạn 2)
- **Tác nhân:** Kỹ sư phần mềm.
- **Mô tả:** Chạy công cụ phân tích log trên các request đã thu thập để xác định tại sao một số request bị cache miss.
- **Tiêu chí nghiệm thu:**
  1. Lệnh `dcp analyze` quét các request trong cùng một session.
  2. Xuất báo cáo chỉ rõ vị trí gãy tiền tố đầu tiên (Prefix Break Point): chỉ mục message, loại block (system, tool, user) và diff cụ thể giữa 2 request liên tiếp.
  3. Gom nhóm theo bảng mã lỗi M1 - M6.

### UC-4: Tối ưu hóa Tiền Tố Bằng Transform Pipeline (Giai đoạn 2)
- **Tác nhân:** Hệ thống proxy (tự động khi bật cấu hình).
- **Mô tả:** Khi request đến, proxy chạy request qua pipeline: Canonicalize $\to$ Prefix Align $\to$ Volatile Extract $\to$ Forward.
- **Tiêu chí nghiệm thu:**
  1. Cấu hình hỗ trợ 3 chế độ: `off`, `shadow`, `on`.
  2. Trong chế độ `shadow`: Proxy tính toán hash sau transform và ghi log dự báo, nhưng vẫn forward request gốc lên upstream.
  3. Trong chế độ `on`: Proxy forward request đã qua transform. Upstream trả về kết quả hợp lệ 100%, agent thực thi trơn tru.
  4. Lệnh `dcp report --baseline <ID/DATE> --evaluated <ID/DATE>` chứng minh tỷ lệ Token Cache Hit Rate tăng ít nhất 15 điểm phần trăm trên cùng loại tác vụ.

---

## 5. Ma Trận Rủi Ro & Biện Pháp Giảm Thiểu (Risk Matrix)

| Rủi ro | Mức độ nghiêm trọng | Khả năng xảy ra | Biện pháp giảm thiểu |
|---|---|---|---|
| **R1: Transform làm sai lệch ngữ nghĩa prompt** | Cao | Thấp | 1. Tuyệt đối không xóa nội dung tin nhắn.<br>2. Chỉ sắp xếp key JSON schema của tools và sắp xếp thứ tự các tool theo tên.<br>3. Kiểm thử nghiêm ngặt với bộ Golden Fixtures.<br>4. Cung cấp cờ bật/tắt riêng cho từng bước transform trong config. |
| **R2: Phá vỡ liên kết tool_call và tool response** | Cực cao | Thấp | Trong chuẩn OpenAI, message có `role: "tool"` bắt buộc phải đi kèm `tool_call_id` tương ứng với `tool_calls` của assistant message trước đó. Pipeline transform **không bao giờ thay đổi thứ tự tương đối** giữa assistant message và tool response liền sau. |
| **R3: Rò rỉ thông tin nhạy cảm trong log** | Trung bình | Thấp | 1. Quyết định D4: Tuyệt đối không bao giờ ghi API key hay header Authorization vào file JSONL hay SQLite.<br>2. Quyết định D7: File log được lưu trong thư mục người dùng cá nhân `~/.dcp/` với phân quyền `0600` (chỉ user sở hữu mới có quyền đọc/ghi). |
| **R4: File log JSONL làm đầy dung lượng ổ cứng** | Thấp | Trung bình | 1. File JSONL được chia theo ngày (`requests_YYYY-MM-DD.jsonl`).<br>2. Cung cấp tùy chọn cấu hình `log.retention_days` để tự động dọn dẹp log cũ hơn $N$ ngày. |
| **R5: Proxy crash hoặc upstream lỗi làm gián đoạn coding** | Cao | Thấp | 1. Áp dụng cơ chế fail-open: Nếu SQLite bị lỗi hoặc transform sinh exception, proxy lập tức fallback về passthrough request gốc.<br>2. Trả nguyên trạng status code và error body từ upstream về client. |

---

## 6. Phương Pháp Luận Đo Lường So Sánh Khoa Học

Để bảo đảm tính khách quan khi so sánh hiệu quả giữa Giai đoạn 1 (Baseline) và Giai đoạn 2 (Transform), chúng tôi áp dụng nguyên tắc đo lường đối chứng:

1. **Chuẩn hóa theo Phiên (Session Normalization):**
   - Không so sánh ngẫu nhiên request đơn lẻ.
   - So sánh theo chuỗi turn trong cùng một session làm việc hoặc chạy lại các phiên làm việc giả lập dựa trên bộ dữ liệu kiểm thử chuẩn (Golden Test Fixtures).
2. **Ba chế độ kiểm chứng:**
   - **Baseline (Giai đoạn 1):** Proxy passthrough nguyên bản $100\%$. Thu thập ít nhất 500 requests thực tế từ coding agent.
   - **Shadow Mode (Giai đoạn 2 bước đệm):** Chạy thuật toán transform trong bộ nhớ, tính hash prefix mới, so sánh giả lập với hash của turn trước để ước tính tỷ lệ hit mà không gây rủi ro cho upstream.
   - **Active Transform (Giai đoạn 2 chính thức):** Bật `transform.mode = "on"`, thu thập 500 requests tiếp theo trên cùng tác vụ hoặc replay cùng kịch bản để so sánh định lượng.
3. **Công thức tính mức độ cải thiện:**
   $$\Delta \text{Hit Rate} = \text{Hit Rate}_{\text{phase2}} - \text{Hit Rate}_{\text{phase1}}$$
   $$\% \text{Tiết Kiệm Chi Phí Tăng Thêm} = \frac{\text{Chi Phí}_{\text{phase1}} - \text{Chi Phí}_{\text{phase2}}}{\text{Chi Phí}_{\text{phase1}}} \times 100\%$$
