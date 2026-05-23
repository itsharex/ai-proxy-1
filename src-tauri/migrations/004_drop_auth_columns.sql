-- Drop auth_type and auth_header columns from providers
-- These are now determined automatically from the provider format

-- SQLite 3.35+ supports ALTER TABLE DROP COLUMN
-- For older SQLite, we recreate the table
ALTER TABLE providers DROP COLUMN auth_type;
ALTER TABLE providers DROP COLUMN auth_header;
