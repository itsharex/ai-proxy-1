-- Add format column to providers
ALTER TABLE providers ADD COLUMN format TEXT NOT NULL DEFAULT 'completions'
  CHECK(format IN ('completions', 'responses', 'anthropic', 'gemini'));

-- Create provider_models table
CREATE TABLE IF NOT EXISTS provider_models (
  id TEXT PRIMARY KEY,
  provider_id TEXT NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
  model_name TEXT NOT NULL,
  target_model TEXT,
  enabled INTEGER NOT NULL DEFAULT 1,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE(provider_id, model_name)
);

-- Migrate data from model_routes to provider_models
INSERT OR IGNORE INTO provider_models (id, provider_id, model_name, target_model, enabled, created_at)
SELECT
  id,
  provider_id,
  model_pattern,
  target_model,
  1,
  COALESCE(created_at, datetime('now'))
FROM model_routes;

-- Migrate target_format from model_routes to providers
UPDATE providers SET format = (
  SELECT mr.target_format FROM model_routes mr
  WHERE mr.provider_id = providers.id
  LIMIT 1
) WHERE format = 'completions' AND EXISTS (
  SELECT 1 FROM model_routes WHERE model_routes.provider_id = providers.id
);

-- Drop old tables
DROP TABLE IF EXISTS endpoints;
DROP TABLE IF EXISTS model_routes;
