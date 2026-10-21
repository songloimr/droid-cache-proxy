# Hướng Dẫn Kỹ Sư & Agent (AGENTS.md)

Tài liệu này cung cấp toàn bộ quy ước, lệnh thực thi, kiến trúc thư mục và quy tắc làm việc cho AI agent và lập trình viên khi phát triển dự án `droid-cache-proxy` (`dcp`).

---

## 1. Tổng Quan Dự Án

`droid-cache-proxy` (`dcp`) là một reverse proxy gateway viết bằng Rust, chạy cục bộ tại `127.0.0.1:6789`. Proxy đứng giữa coding agent (ví dụ: Droid, Cline, Aider) và một custom endpoint upstream chuẩn OpenAI-compatible.

### Mục tiêu 2 giai đoạn:
- **Giai đoạn 1 (Thu thập & Đo lường baseline):** Proxy forward toàn bộ request đến upstream endpoint với API key cấu hình sẵn, tự động chèn `stream_options.include_usage=true` khi stream, thu thập log thô vào file JSONL và ghi metadata/metrics/segment hashes vào SQLite. Cung cấp lệnh CLI và HTTP endpoint để đo cache hit rate baseline.
- **Giai đoạn 2 (Phân tích & Tối ưu hóa Cache Hit):** Công cụ phân tích log (CLI analyzer) phát hiện các điểm lệch prefix giữa các request. Pipeline biến đổi (transform pipeline) tự động chuẩn hóa JSON, sắp xếp tool definitions, căn chỉnh thứ tự prefix (static lên trước, dynamic xuống sau), loại bỏ hoặc di dời các trường volatile. Chế độ shadow mode cho phép đo lường trước khi kích hoạt chính thức.

---

## 2. Bản Đồ Thư Mục Dự Án

```text
droid-cache-proxy/
├── AGENTS.md                          # Tài liệu chỉ dẫn này
├── docs/
│   ├── 01-PRD.md                      # Product Requirements Document
│   ├── 02-business-analysis.md        # Phân tích nghiệp vụ & kinh tế học prompt cache
│   ├── 03-technical-spec.md           # Đặc tả kỹ thuật chi tiết
│   ├── 04-contracts/                  # Hợp đồng giao diện (Schema, API, DB)
│   │   ├── config.schema.md           # Cấu hình config.toml
│   │   ├── config.example.toml        # File cấu hình mẫu
│   │   ├── db-schema.sql              # Schema cơ sở dữ liệu SQLite
│   │   ├── jsonl-format.md            # Quy cách log JSONL từng dòng
│   │   └── api.md                     # Đặc tả API forward, health, metrics & CLI
│   └── 05-tasks.md                    # Danh sách task triển khai chi tiết kèm verification
├── fixtures/                          # Dữ liệu mẫu dùng cho unit test & integration test
│   ├── README.md                      # Hướng dẫn sử dụng fixtures
│   ├── requests/                      # Request mẫu (non-stream, stream, tool calls, multi-turn)
│   ├── responses/                     # Response upstream mẫu (JSON, SSE chunks, usage)
│   └── golden/                        # Cặp input/output chuẩn cho từng bộ transform
└── crates/ (khi triển khai code)
    ├── dcp/                           # Binary CLI & entrypoint chính
    ├── dcp-core/                      # Data models, config parser, error types
    ├── dcp-proxy/                     # Axum server, HTTP forwarder, SSE relay
    ├── dcp-store/                     # SQLite writer, JSONL appender, ring buffer
    ├── dcp-transform/                 # Canonicalization, prefix alignment, pipeline
    └── dcp-analyze/                   # Offline log analysis, diff finder, report generator
```

---

## 3. Thứ Tự Đọc Tài Liệu (Reading Order)

Trước khi thực hiện bất kỳ task nào, agent phải đọc tài liệu theo thứ tự sau:
1. `AGENTS.md` (file này): Hiểu quy ước, lệnh và quy tắc an toàn.
2. `docs/01-PRD.md`: Nắm rõ bài toán, KPI và phạm vi từng giai đoạn.
3. `docs/03-technical-spec.md`: Nắm rõ luồng xử lý dữ liệu và cấu trúc các crate.
4. `docs/04-contracts/*`: Nắm vững định dạng dữ liệu đầu vào/ra, config và database.
5. `docs/05-tasks.md`: Chọn task cần làm, tuân thủ đúng Acceptance Criteria.

---

## 4. Lệnh Chuẩn (Standard Commands)

Mọi thay đổi code Rust trong dự án phải kiểm tra bằng các lệnh sau:

### Build & Kiểm tra cú pháp
```bash
# Kiểm tra nhanh không sinh binary
cargo check --workspace --all-targets

# Build debug
cargo build --workspace

# Build release tối ưu hóa tốc độ
cargo build --workspace --release
```

### Kiểm tra định dạng & Linter
```bash
# Kiểm tra format (bắt buộc pass trước khi commit)
cargo fmt --all -- --check

# Format tự động
cargo fmt --all

# Chạy Clippy với cờ nghiêm ngặt (không chấp nhận warning)
cargo clippy --workspace --all-targets -- -D warnings
```

### Chạy Unit Test & Integration Test
```bash
# Chạy toàn bộ test suite
cargo test --workspace

# Chạy test có in log stdout
cargo test --workspace -- --nocapture

# Chạy riêng test transform với golden fixtures
cargo test -p dcp-transform
```

### Chạy ứng dụng Local
```bash
# Khởi động proxy server với file config
cargo run -p dcp -- serve --config config.toml

# Xem thống kê cache hit từ SQLite
cargo run -p dcp -- stats --db ~/.dcp/dcp.db

# Chạy bộ phân tích log giai đoạn 2
cargo run -p dcp -- analyze --db ~/.dcp/dcp.db --jsonl-dir ~/.dcp/logs
```

---

## 5. Quy Ước Lập Trình (Coding Conventions)

### Ngôn ngữ & Stack
- **Rust Edition:** 2021.
- **Async Runtime:** `tokio` (multi-thread).
- **HTTP Framework:** `axum` (v0.7+) hoặc `hyper` (v1.0+).
- **HTTP Client:** `reqwest` (v0.12+) với feature `stream`.
- **JSON Serialization:** `serde`, `serde_json` (bảo toàn thứ tự key khi cần qua `preserve_order`).
- **Database:** `rusqlite` (với bundled SQLite, WAL mode) hoặc `sqlx`.
- **CLI:** `clap` (derive API v4+).

### Xử lý lỗi (Error Handling)
- Trong các thư viện nội bộ (`dcp-core`, `dcp-proxy`, `dcp-store`, `dcp-transform`, `dcp-analyze`): Sử dụng `thiserror` để định nghĩa enum lỗi rõ ràng, có ngữ cảnh.
- Trong ứng dụng CLI (`crates/dcp`): Sử dụng `anyhow::Result` ở tầng `main`.
- **Tuyệt đối không dùng `unwrap()` hoặc `expect()`** trong luồng xử lý request HTTP/SSE của proxy. Mọi lỗi upstream, lỗi parse JSON, hoặc lỗi ghi log đều phải được bắt và xử lý an toàn (fallback về request gốc hoặc ghi log lỗi mà không ngắt luồng người dùng).

### Logging & Tracing
- Sử dụng crate `tracing` thay vì `println!` hay `eprintln!`.
- Sử dụng `tracing::instrument` trên các hàm xử lý request chính.
- **Quy tắc bảo mật:** Không bao giờ ghi `Authorization` header, API key hay raw token vào tracing log ở mức `INFO`, `WARN` hay `ERROR`.

### Tính Bất Biến Của Transform (Transform Invariants)
1. **Tính thuần khiết (Purity):** Cùng một input JSON và cùng một cấu hình transform phải luôn tạo ra output giống hệt nhau từng byte (deterministic).
2. **Tính lũy đẳng (Idempotence):** `Transform(Transform(x)) == Transform(x)`. Chạy transform lần thứ hai trên output không được làm thay đổi dữ liệu.
3. **Bảo toàn ngữ nghĩa (Semantic Preservation):** Không được xóa message, không được đảo lộn thứ tự hội thoại giữa `user` và `assistant`, không được tách rời cặp `tool_calls` và `tool` response.
4. **Fail-safe Fallback:** Nếu bất kỳ bước transform nào phát sinh lỗi JSON parsing hoặc vi phạm cấu trúc, proxy phải lập tức chuyển sang chế độ passthrough (dùng nguyên gốc request ban đầu) và ghi nhận cờ `transform_error`.

---

## 6. Định Nghĩa Hoàn Thành (Definition of Done - DoD)

Một task được coi là hoàn thành khi thỏa mãn toàn bộ các điều kiện sau:
1. Mã nguồn thỏa mãn `cargo fmt --all -- --check` và `cargo clippy --workspace --all-targets -- -D warnings`.
2. Toàn bộ unit tests và integration tests liên quan đều PASS (`cargo test`).
3. Nếu thay đổi cấu trúc dữ liệu, cấu hình hoặc API, tài liệu tương ứng trong `docs/04-contracts/` đã được cập nhật đồng bộ.
4. Acceptance criteria nêu trong `docs/05-tasks.md` được kiểm chứng bằng lệnh cụ thể.
