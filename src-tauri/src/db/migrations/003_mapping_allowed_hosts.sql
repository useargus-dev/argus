-- Per-mapping allowed hosts for HTTP MITM proxy
ALTER TABLE bucket_mappings ADD COLUMN allowed_hosts TEXT NOT NULL DEFAULT '[]';

-- Copy bucket-level hosts onto proxy-enabled mappings (one-time migration)
UPDATE bucket_mappings
SET allowed_hosts = (
    SELECT allowed_hosts FROM app_buckets WHERE app_buckets.id = bucket_mappings.bucket_id
)
WHERE proxy_enabled = 1;
