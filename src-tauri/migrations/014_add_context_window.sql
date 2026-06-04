-- Add context_window to provider_models for Codex auto-compact integration
ALTER TABLE provider_models ADD COLUMN context_window INTEGER DEFAULT 272000;
