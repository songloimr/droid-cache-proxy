# Quy Cách File Log JSONL (JSONL Log Format Contract)

Tài liệu này định nghĩa cấu trúc của từng dòng log trong file `~/.dcp/logs/requests_YYYY-MM-DD.jsonl`. Mỗi dòng là một đối tượng JSON độc lập, hợp lệ, không chứa ký tự xuống dòng bên trong giá trị (`\n` được escape thành `\\n`).

---

## 1. Nguyên Tắc Bảo Mật & Quyền Hạn File
1. **Phân quyền File (`0600`):** File log JSONL được tạo mới với phân quyền `chmod 0600` (chỉ tài khoản người dùng đang chạy proxy mới có quyền đọc và ghi).
2. **Loại bỏ Secrets Tuyệt Đối:**
   - Trường `headers` trong request **tuyệt đối không chứa** header `authorization`, `proxy-authorization`, hoặc bất kỳ header nào chứa chữ `key` hoặc `token`.
   - Proxy tự động bóc tách các header này ra trước khi lưu vào JSON.
3. **Không Redaction Payload Message:** Theo quyết định đã chốt, nội dung prompt và message trong `body` được giữ nguyên vẹn 100% để phục vụ phân tích thuật toán diff ở Giai đoạn 2.

---

## 2. Cấu Trúc JSON Schema Của Một Dòng Log

```json
{
  "id": "c73a8f2e-4b19-49c2-9e8a-810f6e52dc71",
  "timestamp": "2026-10-20T14:32:05.123Z",
  "session_id": "9f83c18b760a0a86616a245ee83fa74d47225c567840ecfa578c751280a876a3",
  "method": "POST",
  "path": "/v1/chat/completions",
  "status": 200,
  "stream": true,
  "request": {
    "headers": {
      "content-type": "application/json",
      "user-agent": "droid-coding-agent/1.2.0",
      "accept": "text/event-stream"
    },
    "body": {
      "model": "deepseek-chat",
      "messages": [
        {
          "role": "system",
          "content": "You are an expert software engineer..."
        },
        {
          "role": "user",
          "content": "Refactor this function to improve performance."
        }
      ],
      "tools": [
        {
          "type": "function",
          "function": {
            "name": "read_file",
            "description": "Read file contents",
            "parameters": {
              "type": "object",
              "properties": {
                "path": { "type": "string" }
              },
              "required": ["path"]
            }
          }
        }
      ],
      "stream": true,
      "stream_options": {
        "include_usage": true
      }
    }
  },
  "transformed_request": {
    "applied": false,
    "mode": "off",
    "version": null,
    "error": null,
    "body": null
  },
  "response": {
    "status": 200,
    "headers": {
      "content-type": "text/event-stream; charset=utf-8"
    },
    "stream_summary": {
      "total_chunks": 42,
      "finish_reason": "stop",
      "accumulated_content_length": 850
    }
  },
  "metrics": {
    "prompt_tokens": 15420,
    "cached_tokens": 12288,
    "completion_tokens": 195,
    "total_tokens": 15615,
    "cache_status": "partial",
    "latency_ms": 1420,
    "ttft_ms": 185,
    "proxy_overhead_ms": 1.25
  },
  "prefix": {
    "prefix_hash": "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    "shadow_prefix_hash": null,
    "segment_count": 3
  }
}
```

---

## 3. Giải Thích Chi Tiết Các Khối

### 3.1. Khối `request`
- `headers`: Bản đồ các header hợp lệ đã được lọc bỏ thông tin nhạy cảm.
- `body`: Toàn bộ JSON payload ban đầu do client gửi lên.

### 3.2. Khối `transformed_request`
- Xuất hiện khi `transform.mode != "off"`:
  - `applied`: Boolean (`true` nếu thực sự gửi body sau transform lên upstream, `false` nếu ở chế độ shadow hoặc gặp lỗi).
  - `mode`: `"off"` | `"shadow"` | `"on"`.
  - `version`: Chuỗi định danh phiên bản thuật toán transform, lấy từ `[transform] version` trong config (ví dụ: `"v1.0.0"`).
  - `error`: Một trong các mã: `parse_error | validation_failed | non_idempotent | custom_rule_error | volatile_extract_error` (định nghĩa tại `docs/03-technical-spec.md` §5.2). `null` khi không có lỗi.
  - `body`: Toàn bộ JSON payload sau khi đã đi qua pipeline transform.

### 3.3. Khối `response`
- Nếu là request không stream (`stream: false`): Lưu toàn bộ body JSON trả về từ upstream.
- Nếu là request streaming (`stream: true`): Không lưu toàn bộ hàng chục chuỗi `data: {...}` rời rạc để tránh lãng phí dung lượng đĩa. Thay vào đó, lưu `stream_summary` gồm:
  - `total_chunks`: Tổng số chunk SSE nhận được.
  - `finish_reason`: Giá trị kết thúc stream (`"stop"`, `"tool_calls"`, v.v.).
  - `accumulated_content_length`: Tổng số ký tự văn bản đã relay về cho client.

### 3.4. Khối `metrics`
- Tập hợp số liệu phục vụ truy vấn nhanh mà không cần parse lại SQLite:
  - `prompt_tokens`, `cached_tokens`, `completion_tokens`, `total_tokens`.
  - `cache_status`: Phân loại trạng thái hit (`"hit"`, `"partial"`, `"miss"`, `"sub_threshold"`, `"unknown"`).
  - `latency_ms`: Tổng thời gian xử lý toàn trình (ms).
  - `ttft_ms`: Thời gian tới chunk đầu tiên (ms).
  - `proxy_overhead_ms`: Thời gian xử lý tiêu tốn bởi code Rust của proxy (ms).
