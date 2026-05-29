-- Add optional endpoint_path column to providers table.
-- NULL = use default path based on format (backward compatible).
-- Non-NULL = use this custom path instead.
ALTER TABLE providers ADD COLUMN endpoint_path TEXT;
