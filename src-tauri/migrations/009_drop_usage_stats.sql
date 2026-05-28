-- Drop usage_stats table (statistics now derived from request_logs)
DROP TABLE IF EXISTS usage_stats;

-- Add composite index for statistics queries
CREATE INDEX IF NOT EXISTS idx_request_logs_status_created
ON request_logs(status_code, created_at);
