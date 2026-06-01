-- Per-bucket HTTP MITM proxy settings
ALTER TABLE app_buckets ADD COLUMN proxy_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE app_buckets ADD COLUMN proxy_port INTEGER;
ALTER TABLE app_buckets ADD COLUMN allowed_hosts TEXT NOT NULL DEFAULT '[]';

ALTER TABLE bucket_mappings ADD COLUMN proxy_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE bucket_mappings ADD COLUMN proxy_placeholder TEXT;
