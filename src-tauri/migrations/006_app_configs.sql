CREATE TABLE IF NOT EXISTS app_configs (
    app_type TEXT PRIMARY KEY,
    model TEXT NOT NULL,
    proxy_url TEXT NOT NULL,
    launched_at TEXT NOT NULL,
    config_path TEXT,
    install_path TEXT,
    status TEXT NOT NULL DEFAULT 'success'
);
