-- Add timeout settings
INSERT OR IGNORE INTO settings (key, value) VALUES ('request_timeout', '300');
INSERT OR IGNORE INTO settings (key, value) VALUES ('connect_timeout', '30');
