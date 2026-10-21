# Tài Liệu Yêu Cầu Sản Phẩm (PRD) - Droid Cache Proxy (`dcp`)

- **Tên dự án:** Droid Cache Proxy (`dcp`)
- **Trạng thái:** Bản thảo kỹ thuật (Spec Mode Exit)
- **Phiên bản:** 1.0.0
- **Tác giả:** Kỹ sư hệ thống & AI Agent

---

## 1. Bối Cảnh & Vấn Đề (Problem Statement)

### 1.1. Bối cảnh
Các AI coding agent hiện đại (như Droid, Cline, Roo Code, Aider, OpenCode) hoạt động theo mô hình tương tác liên tục nhiều vòng (multi-turn agent loop). Trong mỗi vòng:
1. Agent gửi toàn bộ ngữ cảnh bao gồm: System prompt (quy tắc, vai trò, cấu trúc repo), danh sách công cụ (tool definitions), lịch sử hội thoại, và kết quả thực thi công cụ (tool execution outputs).
2. Kích thước prompt thường dao động từ 20.000 đến hơn 150.000 tokens cho mỗi turn.
3. Các LLM provider hiện đại (OpenAI, DeepSeek, Anthropic, Qwen, vLLM, SGLang) đều trang bị cơ chế **Prompt Caching** (KV Cache reuse). Khi prompt mới có một đoạn tiền tố (prefix) trùng khớp hoàn toàn từng token với prompt đã được xử lý trước đó, provider sẽ tái sử dụng KV cache đã tính toán.
4. Lợi ích của Prompt Caching:
   - **Chi phí:** Giảm giá thành từ 50% đến 90% cho phần token được cache hit (ví dụ: OpenAI giảm 50%, DeepSeek giảm tới 90%).
   - **Độ trễ:** Giảm mạnh thời gian xử lý token ban đầu (Time To First Token - TTFT), giúp agent phản hồi nhanh hơn gấp 2 đến 5 lần.

### 1.2. Vấn đề thực tế
Dù prompt của coding agent có tính lặp lại rất cao giữa các turn (thường chỉ thêm 1 message user hoặc 1 kết quả tool ở cuối), **tỉ lệ Cache Hit trong thực tế vẫn rất thấp hoặc thường xuyên bị reset về 0%** do các nguyên nhân:
1. **Volatile Metadata trong System Prompt:** Agent chèn ngày giờ thực tế (`Current Time: 2026-10-20 14:32:05`), PID tiến trình, trạng thái git thay đổi liên tục ngay tại phần đầu của System prompt. Điều này làm thay đổi prefix ngay từ những token đầu tiên, phá vỡ toàn bộ cache.
2. **Thứ tự Tool Definitions không cố định:** Danh sách tools được sinh ra từ các ngôn ngữ/framework có thứ tự key hoặc thứ tự mảng thay đổi ngẫu nhiên giữa các lần gọi hoặc khi bật/tắt extension.
3. **Định dạng JSON không chuẩn hóa (Non-canonical JSON):** Khoảng trắng, ký tự xuống dòng, thứ tự các trường trong JSON schema của tools bị xáo trộn dù nội dung ngữ nghĩa không đổi.
4. **Cấu trúc hội thoại chèn dynamic data lên đầu:** Một số agent chèn thông tin tài liệu mới tra cứu (doc retrieval) hoặc tệp tin vừa đọc lên đầu lịch sử trò chuyện thay vì đưa xuống dưới.

### 1.3. Giải pháp
Xây dựng `droid-cache-proxy` (`dcp`): Một proxy gateway cục bộ cực nhẹ viết bằng Rust, đứng giữa Coding Agent và Custom Upstream Endpoint. Dự án chia làm 2 giai đoạn:
- **Giai đoạn 1:** Chạy passthrough trong suốt, thu thập log chi tiết (SQLite + JSONL), đo lường chính xác tỷ lệ cache hit baseline thực tế.
- **Giai đoạn 2:** Đọc log, phân tích điểm gãy prefix (prefix break analysis), xây dựng bộ transform tiền xử lý request để chuẩn hóa và sắp xếp prefix, tối đa hóa tỉ lệ cache hit và đo lường sự cải thiện.

---

## 2. Mục Tiêu & Chỉ Số Đánh Giá (Goals & KPIs)

### 2.1. Mục tiêu cốt lõi (Goals)
1. **Minh bạch & Đơn giản:** Cấu hình 1 endpoint upstream duy nhất và 1 API key, lắng nghe tại cổng `6789`, forward toàn bộ traffic mà không cần thiết lập routing phức tạp.
2. **Hiệu năng cao & Zero-overhead:** Viết bằng Rust; độ trễ xử lý thêm của proxy (proxy overhead) duy trì P99 < 5ms.
3. **Đo lường định lượng chính xác (Giai đoạn 1):** Ghi nhận đầy đủ token cache hit, prompt tokens, completion tokens, latency, TTFT từ upstream cho cả luồng streaming (SSE) và non-streaming.
4. **Tối ưu hóa Cache Hit có kiểm chứng (Giai đoạn 2):** Áp dụng pipeline transform bảo toàn tuyệt đối ngữ nghĩa, giúp tăng tỷ lệ token cache hit một cách có thể đo lường.

### 2.2. Non-Goals (Những gì dự án KHÔNG làm)
1. **Không làm Multi-tenant / Multi-user:** Proxy chỉ phục vụ cục bộ cho 1 người dùng trên máy cá nhân (`127.0.0.1`).
2. **Không làm Model Routing / Fallback Provider:** Không định tuyến request qua nhiều provider khác nhau (không thay thế LiteLLM / OpenRouter).
3. **Không làm Prompt Rewrite thay đổi ngữ nghĩa:** Không tóm tắt nội dung, không xóa tin nhắn của người dùng, không can thiệp vào mã nguồn code trong prompt.
4. **Không cung cấp giao diện Web phức tạp:** Chỉ sử dụng CLI (`dcp stats`, `dcp analyze`) và HTTP metrics endpoint đơn giản (`GET /_dcp/metrics`).

### 2.3. Chỉ số đo lường chính (KPIs)

| Chỉ số | Công thức / Định nghĩa | Mục tiêu Giai đoạn 1 | Mục tiêu Giai đoạn 2 |
|---|---|---|---|
| **Token Cache Hit Rate** | $\frac{\sum \text{cached\_tokens}}{\sum \text{prompt\_tokens}} \times 100\%$ | Đo lường baseline thực tế (thường 20% - 45%) | Tăng **+15% đến +35%** so với baseline (đạt 65% - 85%+) |
| **Request Cache Hit Rate** | $\frac{\text{Số request có } \text{cached\_tokens} > 0}{\text{Tổng số request}} \times 100\%$ | Đo lường baseline thực tế | Tăng **+20%** so với baseline |
| **Proxy Overhead Latency** | Thời gian xử lý tại proxy (loại trừ thời gian upstream) | P95 < 2ms, P99 < 5ms | P95 < 3ms, P99 < 5ms |
| **Ước tính Chi Phí Tiết Kiệm** | $\sum (\text{cached\_tokens} \times (\text{Price}_{\text{prompt}} - \text{Price}_{\text{cached}}))$ | Ghi nhận chi phí baseline | Giảm thêm từ 30% đến 60% tổng hóa đơn prompt |
| **Tỷ lệ Lỗi Proxy (Error Rate)** | $\frac{\text{Số lỗi do proxy gây ra}}{\text{Tổng số request}} \times 100\%$ | **0.00%** (Fallback an toàn nếu có lỗi) | **0.00%** (Fallback an toàn nếu transform lỗi) |

---

## 3. Chân Dung Người Dùng (Target Persona)

- **Đối tượng:** Kỹ sư phần mềm sử dụng AI coding agent (như Factory Droid, Cline, Cursor, Aider) để phát triển phần mềm hàng ngày.
- **Môi trường hoạt động:** Máy trạm cá nhân (macOS / Linux / WSL).
- **Hành vi sử dụng:** Mở terminal hoặc IDE, chạy coding agent với cờ `--openai-base-url http://127.0.0.1:6789/v1`.
- **Nhu cầu:** Tiết kiệm chi phí token khi sử dụng các mô hình đắt tiền, giảm độ trễ phản hồi của agent trong các phiên làm việc kéo dài hàng giờ với ngữ cảnh mã nguồn lớn.

---

## 4. Quyết Định & Giả Định Thiết Kế Đã Chốt

| ID | Vấn đề | Quyết định đã phê duyệt |
|---|---|---|
| **D1** | **Upstream Endpoint** | Một endpoint tùy chỉnh duy nhất (`upstream.base_url`) theo chuẩn OpenAI-compatible. |
| **D2** | **Cổng Lắng Nghe** | Lắng nghe tại `127.0.0.1:6789`. Chỉ bind localhost, không yêu cầu xác thực phía client. |
| **D3** | **Phạm vi Forward** | Forward minh bạch toàn bộ các path/method HTTP (`/v1/*`). Chỉ parse chuyên sâu và đo cache hit cho `POST /v1/chat/completions`. Các path khác (`/v1/models`, v.v.) chỉ forward và ghi log metadata cơ bản. |
| **D4** | **Quản lý API Key** | Proxy luôn ghi đè header `Authorization: Bearer <key từ config>` trước khi gửi sang upstream. API key được lấy từ biến môi trường (mặc định `DCP_API_KEY`). Client gửi key nào cũng sẽ được thay thế bằng key chuẩn này. API key **không bao giờ** được ghi vào file log hay database. |
| **D5** | **Xử lý Streaming & Usage** | Khi request có `stream: true`, proxy tự động chèn `stream_options.include_usage=true` vào body gửi sang upstream để nhận chunk thống kê token cuối cùng. Nếu client ban đầu không yêu cầu `include_usage`, proxy sẽ đọc usage để ghi log rồi **lọc bỏ chunk usage này** trước khi forward về client, bảo đảm tương thích 100% với client. |
| **D6** | **Trường dữ liệu Cache Hit** | Đọc ưu tiên từ `usage.prompt_tokens_details.cached_tokens` (chuẩn OpenAI), fallback sang `usage.prompt_cache_hit_tokens` (chuẩn DeepSeek). Nếu upstream không trả về trường nào, đánh dấu trạng thái cache là `unknown` (không coi là miss). |
| **D7** | **Lưu trữ Log** | Sử dụng SQLite (`~/.dcp/dcp.db`) lưu trữ bảng metadata, metrics và hash của từng segment. Lưu trữ file JSONL quay vòng theo ngày (`~/.dcp/logs/requests_YYYY-MM-DD.jsonl`) chứa toàn bộ payload thô không mã hóa/redaction (phục vụ phân tích offline). |
| **D8** | **Ngôn ngữ & Công nghệ** | Ngôn ngữ Rust (Tokio, Axum, Reqwest, Serde, Rusqlite, Clap). |
| **D9** | **Cơ chế An toàn Transform** | Pipeline transform ở Giai đoạn 2 hỗ trợ 3 chế độ: `off` (tắt), `shadow` (tính hash và mô phỏng nhưng vẫn gửi request gốc lên upstream), và `on` (áp dụng request sau transform). Mọi lỗi transform đều tự động fallback về request gốc (fail-open). |

---

## 5. Lộ Trình 2 Giai Đoạn (Two-Phase Scope)

### 5.1. Giai đoạn 1: Thu thập Log & Đo Cache Hit Baseline
- **Trọng tâm:** Xây dựng proxy gateway trong suốt, ổn định tuyệt đối, không làm đứt gãy luồng công việc hiện tại của lập trình viên.
- **Tính năng cần đạt:**
  1. Proxy HTTP server lắng nghe tại `127.0.0.1:6789`.
  2. Forward toàn bộ request tới upstream, inject API key chuẩn từ config.
  3. Xử lý streaming SSE mượt mà, inject và extract `include_usage` an toàn.
  4. Hệ thống ghi log bất đồng bộ (Ring buffer / mpsc channel):
     - Ghi metadata, metrics, segment hashes vào SQLite.
     - Ghi full payload request/response vào JSONL.
  5. Công cụ CLI `dcp stats` hiển thị bảng thống kê: số request, tổng prompt tokens, tổng cached tokens, tỷ lệ cache hit theo ngày/theo model.
  6. Endpoint `GET /_dcp/metrics` xuất báo cáo dưới dạng JSON cho agent hoặc script kiểm tra.
- **Đầu ra:** Tập dữ liệu log thực tế sau 1-2 tuần sử dụng coding agent thông thường để thiết lập chỉ số baseline.

### 5.2. Giai đoạn 2: Phân tích & Tối ưu hóa Cache Hit (Transform Pipeline)
- **Trọng tâm:** Khai thác dữ liệu log giai đoạn 1, tìm ra các điểm làm gãy prefix, xây dựng pipeline biến đổi tự động và đo lường sự khác biệt.
- **Tính năng cần đạt:**
  1. **CLI Analyzer (`dcp analyze`):**
     - Đọc log từ SQLite và JSONL, nhóm theo phiên làm việc (`session_id`).
     - So sánh tiền tố giữa các request liên tiếp $R_n$ và $R_{n+1}$.
     - Định vị byte offset và token đầu tiên bị lệch (Prefix Divergence Point).
     - Phân loại nguyên nhân gãy prefix: Thay đổi thứ tự tools, thay đổi timestamp, khoảng trắng JSON, thay đổi system prompt.
  2. **Transform Pipeline:**
     - `canonicalize`: Chuẩn hóa JSON schema của tools (sắp xếp key alphabet, loại bỏ khoảng trắng dư thừa, chuẩn hóa Unicode NFC), sắp xếp mảng `tools` theo tên tool (`name`).
     - `prefix_align`: Căn chỉnh cấu trúc prompt theo thứ tự tĩnh đến động: Static (System Prompt chuẩn, Tools, Few-shot Examples) $\to$ Semi-static (Lịch sử hội thoại cũ) $\to$ Dynamic (Tin nhắn người dùng mới nhất, kết quả tool vừa chạy). Bảo toàn tuyệt đối thứ tự lượt đối thoại và cặp `tool_calls`/`tool`.
     - `volatile_extract` (Tùy chọn cấu hình): Phát hiện các trường biến thiên theo thời gian (như timestamp, git status, sha) trong system prompt và di dời chúng xuống phần ngữ cảnh động phía dưới.
  3. **Chế độ kiểm thử Shadow Mode:**
     - Cho phép chạy song song: Thực thi transform trên bộ nhớ, tính toán hash prefix mới, so sánh giả lập nhưng vẫn gửi request gốc lên upstream để kiểm tra độ ổn định và đo lường tỷ lệ hit dự kiến.
  4. **Báo cáo So Sánh Định Lượng:**
     - Chạy lệnh `dcp report --baseline <ID/DATE> --evaluated <ID/DATE>` để xuất bảng đối chiếu trước và sau khi kích hoạt transform.

---

## 6. Tiêu Chí Nghiệm Thu Sản Phẩm (Acceptance Criteria)

1. **AC-PRD-01 (Tính trong suốt):** Coding agent trỏ `base_url` về `http://127.0.0.1:6789/v1` hoạt động bình thường, không gặp lỗi giao thức, không bị ngắt stream, kết quả sinh mã trả về đầy đủ.
2. **AC-PRD-02 (Đo lường chính xác):** Sau 100 requests mẫu từ agent, lệnh `dcp stats` hiển thị chính xác tổng số token, cached token khớp với số liệu trên dashboard của provider upstream.
3. **AC-PRD-03 (Bảo toàn ngữ nghĩa):** Ở Giai đoạn 2 khi bật transform, mã sinh ra bởi agent và phản hồi của upstream không bị suy giảm chất lượng logic; 100% các cặp `tool_call` và `tool` result duy trì đúng liên kết `tool_call_id`.
4. **AC-PRD-04 (Hiệu quả tăng trưởng):** Trên cùng một bộ fixture kiểm thử gồm các phiên hội thoại agent 10 turns, chế độ transform mang lại mức tăng Token Cache Hit Rate tối thiểu 15 điểm phần trăm so với chế độ passthrough.
5. **AC-PRD-05 (An toàn lỗi):** Nếu proxy gặp sự cố ngắt kết nối database hoặc file log bị khóa, proxy vẫn tiếp tục forward request bình thường, không làm gián đoạn người dùng.
