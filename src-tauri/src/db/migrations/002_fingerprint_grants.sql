-- Migration 002: Replace uri_hash with fingerprint in client_grants.
-- Drop old table and recreate with fingerprint-based unique constraint.

DROP TABLE IF EXISTS client_grants;

CREATE TABLE IF NOT EXISTS client_grants (
  id              TEXT PRIMARY KEY,
  bucket_id       TEXT NOT NULL REFERENCES app_buckets(id) ON DELETE CASCADE,
  fingerprint     TEXT NOT NULL,
  token_hash      TEXT NOT NULL,
  client_label    TEXT,
  granted_at      TEXT NOT NULL,
  expires_at      TEXT NOT NULL,
  last_seen_at    TEXT,
  UNIQUE(bucket_id, fingerprint, token_hash)
);
