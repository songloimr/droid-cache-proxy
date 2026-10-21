# Hợp Đồng Giao Diện API & CLI (API & CLI Contract)

Tài liệu này định nghĩa chi tiết các giao diện lập trình (HTTP endpoints) và giao diện dòng lệnh (CLI commands) của `droid-cache-proxy` (`dcp`).

---

## 1. Hành Vi Chuyển Tiếp HTTP (Forwarding Behavior)

Proxy lắng nghe tại địa chỉ `127.0.0.1:6789`. Mọi request gửi tới proxy đều được xử lý theo các quy tắc sau:

### 1.1. Bảng phân phối đường dẫn (Route Mapping)

| Method | Path Pattern | Hành vi xử lý |
|---|---|---|
| `GET` | `/_dcp/health` | **Nội bộ:** Kiểm tra trạng thái proxy và kết nối upstream. Không forward. |
| `GET` | `/_dcp/metrics` | **Nội bộ:** Trả về báo cáo metrics và thống kê cache hit dạng JSON. Không forward. |
| `POST` | `/v1/chat/completions` | **Xử lý chuyên sâu:** Phân tích body, bóc tách segments, tính hash, chèn `stream_options.include_usage`, đo latency/TTFT, relay SSE, ghi log SQLite & JSONL. |
| `*` | `/v1/*` | **Chuyển tiếp trong suốt (Transparent Forwarding):** Ghi đè header `Authorization`, thay thế `Host`, forward nguyên trạng body và query params sang upstream. Ghi log metadata cơ bản. |
| `*` | `/*` (Khác) | Chuyển tiếp sang upstream tương ứng theo đường dẫn gốc. |

### 1.2. Quy tắc xử lý Header (Header Manipulation Rules)
1. **Loại bỏ Hop-by-hop Headers:**
   Proxy tự động loại bỏ các header sau trước khi gửi request tới upstream và trước khi trả response về client:
   `Connection`, `Keep-Alive`, `Proxy-Authenticate`, `Proxy-Authorization`, `TE`, `Trailers`, `Transfer-Encoding`, `Upgrade`.
2. **Ghi đè Header Xác thực:**
   `Authorization: Bearer <KEY_FROM_CONFIG_OR_ENV>` luôn thay thế giá trị do client gửi lên.
3. **Thay thế Header Host:**
   Header `Host` được cập nhật thành host của `upstream.base_url`.
4. **Header Nhận diện Session:**
   Proxy đọc header `x-dcp-session` (nếu client cung cấp) để dùng làm `session_id`. Nếu không có, proxy tự động tính hash định danh session.

### 1.3. Cơ chế xử lý lỗi từ Upstream
- Nếu Upstream trả về mã lỗi HTTP ($400, 401, 403, 404, 429, 500, 502, 503$), proxy sẽ chuyển tiếp **chính xác mã HTTP status code** và **nguyên văn nội dung body** về client.
- Nếu không thể kết nối tới upstream (Network Unreachable / Connection Refused): Trả về `502 Bad Gateway` kèm JSON:
  ```json
  {
    "error": {
      "message": "Không thể kết nối tới upstream endpoint",
      "type": "bad_gateway",
      "code": "upstream_unreachable"
    }
  }
  ```
- Nếu quá thời gian `timeout_secs`: Trả về `504 Gateway Timeout` kèm JSON:
  ```json
  {
    "error": {
      "message": "Upstream timeout sau 300 giây",
      "type": "timeout",
      "code": "gateway_timeout"
    }
  }
  ```

---

## 2. Các Endpoint Nội Bộ Của Proxy (Internal Endpoints)

### 2.1. `GET /_dcp/health`
Dùng để kiểm tra tiến trình proxy đang chạy và kết nối tới upstream có thông suốt hay không.

- **Response `200 OK` (Khỏe mạnh):**
  ```json
  {
    "status": "ok",
    "version": "1.0.0",
    "uptime_seconds": 3600,
    "upstream": {
      "base_url": "https://api.deepseek.com/v1",
      "reachable": true
    },
    "storage": {
      "sqlite_ok": true,
      "jsonl_ok": true,
      "pending_queue_size": 2
    }
  }
  ```
- **Response `503 Service Unavailable`:** Trả về khi database bị khóa hoặc không thể ghi file log.

### 2.2. `GET /_dcp/metrics`
Cung cấp số liệu thống kê cache hit tức thời dưới dạng JSON cho agent hoặc dashboard.

- **Response `200 OK`:**
  ```json
  {
    "period": {
      "start_time": "2026-10-20T00:00:00Z",
      "end_time": "2026-10-20T14:45:00Z"
    },
    "summary": {
      "total_requests": 240,
      "stream_requests": 235,
      "non_stream_requests": 5,
      "total_prompt_tokens": 8500400,
      "total_cached_tokens": 5950280,
      "total_completion_tokens": 125400,
      "token_cache_hit_rate_pct": 70.0,
      "request_cache_hit_rate_pct": 82.5
    },
    "latency": {
      "p50_ms": 850,
      "p95_ms": 2400,
      "p99_ms": 4100,
      "avg_proxy_overhead_ms": 1.15
    },
    "by_model": [
      {
        "model": "deepseek-chat",
        "requests": 240,
        "prompt_tokens": 8500400,
        "cached_tokens": 5950280,
        "hit_rate_pct": 70.0
      }
    ]
  }
  ```

---

## 3. Đặc Tả Giao Diện Dòng Lệnh (CLI Interface Contract)

Binary `dcp` cung cấp các lệnh sau:

### 3.1. Lệnh `dcp serve` (Khởi động Proxy Gateway)

```bash
dcp serve [OPTIONS]
```

**Các tùy chọn:**
- `-c, --config <PATH>`: Đường dẫn file cấu hình (mặc định: tìm `./config.toml` hoặc `~/.dcp/config.toml`).
- `-l, --listen <ADDR>`: Ghi đè địa chỉ bind (ví dụ: `127.0.0.1:6789`).
- `-m, --mode <MODE>`: Ghi đè chế độ transform (`off`, `shadow`, `on`).
- `-v, --verbose`: Bật log chi tiết `DEBUG`.

**Mã thoát (Exit Codes):**
- `0`: Dừng tiến trình bình thường qua tín hiệu SIGINT (Ctrl+C) hoặc SIGTERM.
- `1`: Lỗi runtime không mong muốn.
- `2`: Lỗi file cấu hình hoặc thiếu biến môi trường API key.
- `3`: Cổng socket đã bị chiếm dụng (Address already in use).

### 3.2. Lệnh `dcp stats` (Xem Báo Cáo Cache Hit Baseline)

```bash
dcp stats [OPTIONS]
```

**Các tùy chọn:**
- `--db <PATH>`: Đường dẫn file SQLite (mặc định: `~/.dcp/dcp.db`).
- `--days <N>`: Thống kê trong $N$ ngày gần nhất (mặc định: 7).
- `--model <NAME>`: Lọc theo tên model cụ thể.
- `--json`: Xuất kết quả dưới dạng JSON (dành cho automation).

**Mẫu hiển thị Terminal chuẩn:**
```text
================================================================================
                    DROID CACHE PROXY - BÁO CÁO CACHE HIT
================================================================================
Khoảng thời gian: 7 ngày gần nhất (2026-10-13 đến 2026-10-20)
Cơ sở dữ liệu:    ~/.dcp/dcp.db

TỔNG QUAN:
--------------------------------------------------------------------------------
Tổng số request:              1,245
Request streaming:            1,210  (97.2%)
Tổng Prompt Tokens:       34,520,100
Tổng Cached Tokens:       22,438,065
Tổng Completion Tokens:      450,210

CHỈ SỐ HIỆU QUẢ CACHE:
--------------------------------------------------------------------------------
Token Cache Hit Rate:         65.00%  [█████████████░░░░░░░]
Request Cache Hit Rate:       78.40%  (976 / 1,245 requests)
Độ trễ trung bình Proxy:       1.24 ms

PHÂN BỔ THEO MODEL:
--------------------------------------------------------------------------------
Model              Requests    Prompt Tokens    Cached Tokens    Hit Rate (%)
deepseek-chat         1,100       30,120,000       20,481,600          68.00%
gpt-4o                  145        4,400,100        1,956,465          44.46%
================================================================================
```

### 3.3. Lệnh `dcp analyze` (Phân Tích Điểm Gãy Tiền Tố - Giai đoạn 2)

```bash
dcp analyze [OPTIONS]
```

**Các tùy chọn:**
- `--db <PATH>`: Đường dẫn SQLite DB.
- `--jsonl-dir <PATH>`: Thư mục chứa các file log JSONL.
- `--session <SESSION_ID>`: Chỉ phân tích một phiên cụ thể (mặc định: phân tích tất cả các phiên trong 24 giờ qua).
- `--top <N>`: Hiển thị $N$ nguyên nhân gãy prefix phổ biến nhất (mặc định: 10).
- `--format <text|json|markdown>`: Định dạng báo cáo xuất ra.

**Mẫu hiển thị Terminal phân tích:**
```text
================================================================================
              PHÂN TÍCH NGUYÊN NHÂN GÃY TIỀN TỐ (PREFIX DIVERGENCE)
================================================================================
Tổng số phiên phân tích: 35 sessions (540 request transitions)

TOP NGUYÊN NHÂN CACHE MISS PHÁT HIỆN ĐƯỢC:
--------------------------------------------------------------------------------
1. [M1_VOLATILE_TIMESTAMP] Timestamp thay đổi trong System Prompt
   - Số lần phát hiện: 182 lần (33.7%)
   - Vị trí gãy: Segment 0 (System Message)
   - Pattern: "Current Time: 2026-10-20T..."

2. [M2_TOOL_ORDER] Mảng Tools bị xáo trộn thứ tự
   - Số lần phát hiện: 94 lần (17.4%)
   - Vị trí gãy: Segment 1 (Tools Definition)
   - Chi tiết: Danh sách 14 tools đảo vị trí ngẫu nhiên giữa turn N và N+1

3. [M3_NON_CANONICAL_JSON] Key order trong parameter JSON schema
   - Số lần phát hiện: 65 lần (12.0%)
   - Vị trí gãy: Segment 1 (Tools Definition)

4. [M5_TTL_EXPIRY] Khoảng cách giữa 2 turn > 10 phút
   - Số lần phát hiện: 41 lần (7.6%)
================================================================================
```

### 3.4. Lệnh `dcp report` (So Sánh Hiệu Quả Giai Đoạn 1 vs Giai Đoạn 2)

```bash
dcp report --baseline <DATE_OR_SESSION> --evaluated <DATE_OR_SESSION>
```

Xuất báo cáo định lượng đối chiếu tỷ lệ Token Cache Hit Rate, chi phí ước tính tiết kiệm được và độ trễ phản hồi trước và sau khi kích hoạt bộ Transform Pipeline.
