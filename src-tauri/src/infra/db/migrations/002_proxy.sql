-- Per-bucket HTTP MITM proxy settings
ALTER TABLE app_buckets ADD COLUMN proxy_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE app_buckets ADD COLUMN proxy_port INTEGER;
ALTER TABLE app_buckets ADD COLUMN allowed_hosts TEXT NOT NULL DEFAULT '[]';

ALTER TABLE bucket_mappings ADD COLUMN proxy_enabled INTEGER NOT NULL DEFAULT 0;
ALTER TABLE bucket_mappings ADD COLUMN proxy_placeholder TEXT;
ALTER TABLE bucket_mappings ADD COLUMN allowed_hosts TEXT NOT NULL DEFAULT '[]';

-- Copy bucket-level hosts onto proxy-enabled mappings (one-time migration)
UPDATE bucket_mappings
SET allowed_hosts = (
    SELECT allowed_hosts FROM app_buckets WHERE app_buckets.id = bucket_mappings.bucket_id
)
WHERE proxy_enabled = 1;
