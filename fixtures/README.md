# Thư Mục Dữ Liệu Kiểm Thử Mẫu (Test Fixtures)

Thư mục này chứa các dữ liệu mẫu chuẩn (mock datasets) dùng cho Unit Tests, Integration Tests và Golden Tests của dự án `droid-cache-proxy` (`dcp`).

---

## 1. Cấu Trúc Thư Mục

- `fixtures/requests/`: Các payload JSON request mẫu do Coding Agent (Droid, Cline, Aider) gửi tới endpoint `/v1/chat/completions`.
  - `01_chat_simple_non_stream.json`: Request đơn giản không stream, không tools.
  - `02_chat_with_tools_stream.json`: Request stream có chứa danh sách tools và chưa có `stream_options.include_usage`.
  - `03_chat_multi_turn_with_tool_results.json`: Request nhiều lượt đối thoại có cặp `tool_calls` và `tool` response.
  - `04_chat_sub_threshold.json`: Request ngắn (< 1024 tokens) dùng để kiểm tra phân loại `sub_threshold`.
- `fixtures/responses/`: Các payload phản hồi mẫu từ upstream OpenAI-compatible.
  - `01_response_openai_cached.json`: Response JSON chuẩn OpenAI chứa `usage.prompt_tokens_details.cached_tokens`.
  - `02_response_deepseek_cached.json`: Response JSON chuẩn DeepSeek chứa `usage.prompt_cache_hit_tokens`.
  - `03_sse_stream_chunks.txt`: Luồng sự kiện SSE thô (Server-Sent Events) kết thúc bằng chunk chứa `usage` và `data: [DONE]`.
  - `04_response_no_usage.json`: Response không có block usage để kiểm tra trạng thái `unknown`.
- `fixtures/golden/`: Cặp file `_input.json` và `_expected.json` dùng để xác thực tính đúng đắn và tính lũy đẳng ($T(T(x)) == T(x)$) của các thuật toán transform ở Giai đoạn 2.
  - `01_canonicalize_input.json` / `01_canonicalize_expected.json`: Kiểm thử sắp xếp key alphabet và sort mảng tools theo name.
  - `02_prefix_align_input.json` / `02_prefix_align_expected.json`: Kiểm thử căn chỉnh thứ tự khối System $\to$ Tools $\to$ History mà vẫn bảo toàn liên kết `tool_call_id`.

---

## 2. Hướng Dẫn Sử Dụng Trong Test Suite (Rust)

```rust
// Ví dụ nạp request fixture trong test:
let req_json = include_str!("../../../fixtures/requests/02_chat_with_tools_stream.json");
let req: ChatCompletionRequest = serde_json::from_str(req_json).unwrap();

// Ví dụ kiểm thử Golden Test cho transform:
let input_str = include_str!("../../../fixtures/golden/01_canonicalize_input.json");
let expected_str = include_str!("../../../fixtures/golden/01_canonicalize_expected.json");

let mut req: ChatCompletionRequest = serde_json::from_str(input_str).unwrap();
Canonicalizer::apply(&mut req).unwrap();

let actual_json = serde_json::to_string_pretty(&req).unwrap();
let expected_json: serde_json::Value = serde_json::from_str(expected_str).unwrap();
let actual_val: serde_json::Value = serde_json::from_str(&actual_json).unwrap();

assert_eq!(actual_val, expected_json, "Output canonicalize không khớp golden file!");
```
