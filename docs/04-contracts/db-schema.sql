-- ==============================================================================
-- Droid Cache Proxy (dcp) - SQLite Database Schema
-- File: ~/.dcp/dcp.db
-- Thiết lập tối ưu hiệu năng: WAL mode và synchronous NORMAL
-- ==============================================================================

PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA foreign_keys = ON;

-- ------------------------------------------------------------------------------
-- 1. Bảng quản lý phiên bản cơ sở dữ liệu (Migrations)
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS _dcp_migrations (
    version INTEGER PRIMARY KEY,
    description TEXT NOT NULL,
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

INSERT OR IGNORE INTO _dcp_migrations (version, description) 
VALUES (1, 'Initial schema for Phase 1 & Phase 2 prompt cache tracking');

-- ------------------------------------------------------------------------------
-- 2. Bảng quản lý phiên làm việc của Agent (Sessions)
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS sessions (
    session_id TEXT PRIMARY KEY,               -- Hash SHA-256 của (system + tools + first_user_msg) hoặc x-dcp-session header
    model TEXT NOT NULL,                       -- Tên model được sử dụng chính trong session
    first_seen_at TIMESTAMP NOT NULL,          -- Thời điểm bắt đầu session
    last_seen_at TIMESTAMP NOT NULL,           -- Thời điểm request gần nhất trong session
    request_count INTEGER DEFAULT 0,           -- Tổng số request thuộc session
    total_prompt_tokens INTEGER DEFAULT 0,     -- Tổng prompt tokens tích lũy
    total_cached_tokens INTEGER DEFAULT 0,     -- Tổng cached tokens tích lũy
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_sessions_last_seen ON sessions(last_seen_at);

-- ------------------------------------------------------------------------------
-- 3. Bảng lưu trữ chi tiết từng Request (Requests)
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS requests (
    id TEXT PRIMARY KEY,                       -- UUID v4 định danh duy nhất cho request
    timestamp TIMESTAMP NOT NULL,              -- Thời điểm proxy tiếp nhận request (UTC ISO 8601)
    session_id TEXT NOT NULL,                  -- Khóa ngoại liên kết tới bảng sessions
    method TEXT NOT NULL,                      -- HTTP Method (POST, GET...)
    path TEXT NOT NULL,                        -- HTTP Path (/v1/chat/completions...)
    status INTEGER NOT NULL,                   -- HTTP Status Code trả về client (200, 400, 500...)
    model TEXT,                                -- Model request yêu cầu (ví dụ: gpt-4o, deepseek-chat)
    stream BOOLEAN NOT NULL DEFAULT 0,         -- 1 nếu là streaming SSE, 0 nếu là JSON thông thường
    
    -- Token Metrics thu thập từ block usage của Upstream
    prompt_tokens INTEGER DEFAULT 0,           -- Số lượng token của prompt đầu vào
    cached_tokens INTEGER DEFAULT 0,           -- Số lượng token được đọc từ cache (cache hit)
    completion_tokens INTEGER DEFAULT 0,       -- Số lượng token do mô hình sinh ra
    total_tokens INTEGER DEFAULT 0,            -- Tổng số token (prompt + completion)
    
    -- Trạng thái Cache Hit phân loại:
    -- 'hit': 100% prompt tokens nằm trong cache
    -- 'partial': cached_tokens > 0 nhưng < prompt_tokens
    -- 'miss': cached_tokens == 0 và prompt_tokens >= 1024
    -- 'sub_threshold': prompt_tokens < 1024 (chưa đủ điều kiện cache của provider)
    -- 'unknown': upstream không trả về thông tin cached_tokens
    cache_status TEXT NOT NULL DEFAULT 'unknown',
    
    -- Metrics về độ trễ (Latency)
    latency_ms INTEGER NOT NULL,               -- Tổng thời gian từ lúc nhận request đến khi gửi xong chunk cuối (ms)
    ttft_ms INTEGER,                           -- Time To First Token: thời gian từ lúc gửi upstream đến chunk đầu (ms)
    proxy_overhead_ms REAL NOT NULL DEFAULT 0, -- Thời gian xử lý riêng của proxy (parse, transform, hash) (ms)
    
    -- Thông tin liên quan đến Transform (Giai đoạn 2)
    transform_mode TEXT NOT NULL DEFAULT 'off',-- 'off' | 'shadow' | 'on'
    transform_version TEXT,                    -- Chuỗi định danh thuật toán (ví dụ: 'v1.0.0-canonical+align')
    transform_error TEXT,                      -- Ghi nhận lỗi nếu bước transform thất bại và phải fallback
    prefix_hash TEXT,                          -- Hash SHA-256 của toàn bộ prefix được gửi lên upstream
    shadow_prefix_hash TEXT,                   -- Hash SHA-256 sau khi transform (khi chạy ở chế độ shadow)
    
    -- Tham chiếu file log JSONL để truy xuất toàn bộ payload thô
    jsonl_file TEXT,                           -- Tên file JSONL (ví dụ: 'requests_2026-10-20.jsonl')
    jsonl_line_number INTEGER,                 -- Số dòng trong file JSONL
    
    FOREIGN KEY(session_id) REFERENCES sessions(session_id)
);

CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_requests_timestamp ON requests(timestamp);
CREATE INDEX IF NOT EXISTS idx_requests_cache_status ON requests(cache_status);
CREATE INDEX IF NOT EXISTS idx_requests_prefix_hash ON requests(prefix_hash);

-- ------------------------------------------------------------------------------
-- 4. Bảng phân rã các Segments của Prompt (Segments - Phục vụ Diff & Divergence)
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS segments (
    request_id TEXT NOT NULL,                  -- Khóa ngoại trỏ về bảng requests
    segment_index INTEGER NOT NULL,            -- Thứ tự segment: 0 = system message đầu tiên (hoặc message đầu nếu không có system), 1 = toàn bộ mảng tools gộp, 2..k = từng message còn lại theo thứ tự
    kind TEXT NOT NULL,                        -- Loại khối: 'system' | 'tools' | 'user' | 'assistant' | 'tool'
    segment_hash TEXT NOT NULL,                -- Hash SHA-256 riêng biệt của khối này
    cumulative_hash TEXT NOT NULL,             -- Hash SHA-256 tích lũy từ segment 0 đến segment này
    token_estimate INTEGER,                    -- Ước lượng số token của segment
    
    PRIMARY KEY(request_id, segment_index),
    FOREIGN KEY(request_id) REFERENCES requests(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_segments_cumulative_hash ON segments(cumulative_hash);
