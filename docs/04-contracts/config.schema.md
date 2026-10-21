# Hợp Đồng Cấu Hình (Configuration Contract) - `config.toml`

Tài liệu này định nghĩa cấu trúc, kiểu dữ liệu, giá trị mặc định và quy tắc kiểm tra tính hợp lệ của file cấu hình `config.toml` cho `droid-cache-proxy`.

---

## 1. Vị Trí File Cấu Hình

Proxy tìm kiếm file cấu hình theo thứ tự ưu tiên:
1. Đường dẫn truyền qua tham số dòng lệnh: `dcp serve --config <PATH>`
2. Biến môi trường: `DCP_CONFIG_PATH`
3. Thư mục hiện tại: `./config.toml`
4. Thư mục cấu hình người dùng: `~/.dcp/config.toml`

Nếu không tìm thấy file nào, proxy sẽ dừng và báo lỗi yêu cầu tạo file cấu hình hoặc sử dụng template mẫu `config.example.toml`.

---

## 2. Đặc Tả Các Trường Trong Cấu Hình

### 2.1. Nhóm `[server]` (Cấu hình Proxy Server)

| Tên trường | Kiểu dữ liệu | Bắt buộc | Mặc định | Mô tả & Quy tắc kiểm tra |
|---|---|---|---|---|
| `listen` | String | Không | `"127.0.0.1:6789"` | Địa chỉ IP và cổng socket để proxy lắng nghe. Phải ở dạng `IP:PORT`. Khuyến nghị chỉ bind `127.0.0.1` để đảm bảo an toàn máy cá nhân. |
| `max_connections` | Integer | Không | `1024` | Số lượng kết nối đồng thời tối đa. Giá trị từ `10` đến `65535`. |

### 2.2. Nhóm `[upstream]` (Cấu hình Endpoint Đích)

| Tên trường | Kiểu dữ liệu | Bắt buộc | Mặc định | Mô tả & Quy tắc kiểm tra |
|---|---|---|---|---|
| `base_url` | String | **Có** | Không có | URL gốc của upstream OpenAI-compatible. Ví dụ: `"https://api.openai.com/v1"`, `"https://api.deepseek.com"`, `"http://10.0.0.5:8000/v1"`. Không được chứa ký tự `/` ở cuối. |
| `api_key_env` | String | Không | `"DCP_API_KEY"` | Tên biến môi trường chứa API key dùng để forward sang upstream. Nếu biến môi trường này không tồn tại khi khởi động, proxy sẽ báo lỗi. |
| `timeout_secs` | Integer | Không | `300` | Thời gian chờ tối đa (timeout) cho một request tới upstream tính bằng giây. Giá trị tối thiểu `10`, tối đa `3600`. |
| `connect_timeout_secs` | Integer | Không | `10` | Thời gian chờ kết nối TCP tới upstream. Giá trị từ `1` đến `60`. |

### 2.3. Nhóm `[stream]` (Cấu hình Xử lý Luồng)

| Tên trường | Kiểu dữ liệu | Bắt buộc | Mặc định | Mô tả & Quy tắc kiểm tra |
|---|---|---|---|---|
| `inject_include_usage` | Boolean | Không | `true` | Khi `true`, proxy sẽ tự động chèn `stream_options.include_usage = true` vào các request streaming để nhận token usage từ upstream. Nếu client không yêu cầu, chunk usage sẽ được trích xuất rồi lọc bỏ trước khi gửi về client. |

### 2.4. Nhóm `[log]` (Cấu hình Lưu Trữ & Thống Kê)

| Tên trường | Kiểu dữ liệu | Bắt buộc | Mặc định | Mô tả & Quy tắc kiểm tra |
|---|---|---|---|---|
| `dir` | String | Không | `"~/.dcp"` | Thư mục gốc để lưu trữ database SQLite và file JSONL. Tự động giải phóng ký tự `~` thành home directory của người dùng hiện tại. |
| `db_filename` | String | Không | `"dcp.db"` | Tên file database SQLite nằm trong `dir`. |
| `jsonl_enabled` | Boolean | Không | `true` | Bật/tắt việc ghi chi tiết toàn bộ payload thô ra file JSONL theo ngày. |
| `channel_capacity` | Integer | Không | `10000` | Dung lượng bộ đệm in-memory channel giữa worker HTTP và worker ghi đĩa. |
| `batch_flush_ms` | Integer | Không | `500` | Khoảng thời gian tối đa để ghi gom batch vào SQLite (milli-giây). |
| `batch_size` | Integer | Không | `50` | Số lượng request tối đa để kích hoạt ghi batch ngay lập tức vào SQLite. |
| `retention_days` | Integer | Không | `30` | Số ngày lưu trữ file log JSONL trước khi tự động xoay vòng dọn dẹp. Đặt `0` để lưu vô hạn. |

### 2.5. Nhóm `[transform]` (Cấu hình Tối Ưu Hóa Tiền Tố - Giai đoạn 2)

| Tên trường | Kiểu dữ liệu | Bắt buộc | Mặc định | Mô tả & Quy tắc kiểm tra |
|---|---|---|---|---|
| `mode` | String | Không | `"off"` | Chế độ hoạt động của pipeline biến đổi. Giá trị hợp lệ: `"off"` (Giai đoạn 1), `"shadow"` (Mô phỏng & ghi log), `"on"` (Áp dụng chính thức). |
| `version` | String | Không | `"v1.0.0"` | Định danh phiên bản thuật toán transform. Giá trị này được ghi vào cột `transform_version` trong SQLite và trường `transformed_request.version` trong JSONL. Không được để rỗng.
| `canonicalize` | Boolean | Không | `true` | Bật/tắt chuẩn hóa JSON schema, alphabet key sort và sort tools theo name. |
| `prefix_align` | Boolean | Không | `true` | Bật/tắt căn chỉnh thứ tự các khối: System $\to$ Tools $\to$ Lịch sử $\to$ Turn hiện tại. |
| `volatile_extract` | Boolean | Không | `false` | Bật/tắt việc bóc tách timestamp và git status ra khỏi System prompt đưa xuống cuối. Mặc định là `false` để đảm bảo tính an toàn. |
| `custom_rules_file` | String | Không | `""` | Đường dẫn tới file định nghĩa rules regex bổ sung (nếu có). |

---

## 3. Quy Tắc Kiểm Tra Tính Hợp Lệ Khi Khởi Động (Validation Rules)

Khi khởi động, `dcp` thực hiện kiểm tra cấu hình nghiêm ngặt (Fail-fast):
1. **Kiểm tra `upstream.base_url`:**
   - Phải là một URL hợp lệ bắt đầu bằng `http://` hoặc `https://`.
   - Nếu URL kết thúc bằng dấu `/`, proxy tự động cắt bỏ và in thông báo chuẩn hóa.
2. **Kiểm tra API Key:**
   - Đọc biến môi trường quy định tại `upstream.api_key_env` (mặc định `DCP_API_KEY`).
   - Nếu biến không tồn tại hoặc chuỗi rỗng: Dừng tiến trình ngay lập tức với mã lỗi `EXIT_CODE_CONFIG_ERROR (2)` và thông báo:
     `"Lỗi: Không tìm thấy API key trong biến môi trường 'DCP_API_KEY'. Hãy chạy 'export DCP_API_KEY=your_key' trước khi khởi động."`
3. **Kiểm tra Quyền Thư Mục Log:**
   - Proxy kiểm tra xem có thể đọc/ghi tại `log.dir` hay không. Nếu thư mục chưa tồn tại, tự động tạo với quyền `0700`.
   - Nếu không thể tạo thư mục do lỗi phân quyền, dừng tiến trình và báo lỗi.
4. **Kiểm tra `transform.mode`:**
   - Chỉ chấp nhận 1 trong 3 giá trị: `"off"`, `"shadow"`, `"on"`. Nếu nhập giá trị khác, proxy dừng với lỗi cú pháp cấu hình.
