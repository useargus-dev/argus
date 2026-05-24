-- Argus base schema. Fresh databases apply this file once (version 1).

CREATE TABLE IF NOT EXISTS schema_migrations (
  version     INTEGER PRIMARY KEY,
  applied_at  TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS users (
  id                  TEXT PRIMARY KEY DEFAULT 'local',
  email               TEXT NOT NULL UNIQUE,
  username            TEXT NOT NULL UNIQUE,
  avatar_url          TEXT,
  password_hash       TEXT NOT NULL,
  totp_secret         TEXT,
  second_factor_type  TEXT NOT NULL,
  totp_enabled        INTEGER NOT NULL DEFAULT 0,
  biometric_enrolled  INTEGER NOT NULL DEFAULT 0,
  created_at          TEXT NOT NULL,
  last_signed_in_at   TEXT
);

CREATE TABLE IF NOT EXISTS settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS secrets (
  id            TEXT PRIMARY KEY,
  name          TEXT NOT NULL,
  secret_type   TEXT NOT NULL,
  organization  TEXT,
  environment   TEXT,
  description   TEXT,
  tags          TEXT,
  value         TEXT NOT NULL,
  expires_at    TEXT,
  is_archived   INTEGER NOT NULL DEFAULT 0,
  created_at    TEXT NOT NULL,
  updated_at    TEXT NOT NULL
);

CREATE VIRTUAL TABLE IF NOT EXISTS secrets_fts USING fts5(
  name,
  description,
  content='secrets',
  content_rowid='rowid'
);

CREATE TRIGGER IF NOT EXISTS secrets_fts_ai AFTER INSERT ON secrets BEGIN
  INSERT INTO secrets_fts(rowid, name, description) VALUES (new.rowid, new.name, new.description);
END;

CREATE TRIGGER IF NOT EXISTS secrets_fts_ad AFTER DELETE ON secrets BEGIN
  INSERT INTO secrets_fts(secrets_fts, rowid, name, description) VALUES ('delete', old.rowid, old.name, old.description);
END;

CREATE TRIGGER IF NOT EXISTS secrets_fts_au AFTER UPDATE ON secrets BEGIN
  INSERT INTO secrets_fts(secrets_fts, rowid, name, description) VALUES ('delete', old.rowid, old.name, old.description);
  INSERT INTO secrets_fts(rowid, name, description) VALUES (new.rowid, new.name, new.description);
END;

CREATE TABLE IF NOT EXISTS app_buckets (
  id                    TEXT PRIMARY KEY,
  name                  TEXT NOT NULL UNIQUE,
  description           TEXT,
  client_token_hash     TEXT NOT NULL,
  client_token_enc      TEXT,
  access_ttl_minutes    INTEGER NOT NULL DEFAULT 60,
  refresh_ttl_minutes   INTEGER,
  session_ttl_minutes   INTEGER NOT NULL DEFAULT 480,
  is_tray_active        INTEGER NOT NULL DEFAULT 1,
  created_at            TEXT NOT NULL,
  updated_at            TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS bucket_mappings (
  id          TEXT PRIMARY KEY,
  bucket_id   TEXT NOT NULL REFERENCES app_buckets(id) ON DELETE CASCADE,
  env_label   TEXT NOT NULL,
  secret_id   TEXT NOT NULL REFERENCES secrets(id) ON DELETE RESTRICT,
  created_at  TEXT NOT NULL,
  UNIQUE(bucket_id, env_label)
);

CREATE TABLE IF NOT EXISTS approvals (
  id              TEXT PRIMARY KEY,
  bucket_id       TEXT REFERENCES app_buckets(id) ON DELETE CASCADE,
  process_path    TEXT NOT NULL,
  working_dir     TEXT NOT NULL,
  process_name    TEXT,
  pid             INTEGER,
  granted_at      TEXT NOT NULL,
  expires_at      TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS client_grants (
  id              TEXT PRIMARY KEY,
  bucket_id       TEXT NOT NULL REFERENCES app_buckets(id) ON DELETE CASCADE,
  uri_hash        TEXT NOT NULL,
  uri_display     TEXT NOT NULL,
  token_hash      TEXT NOT NULL,
  client_label    TEXT,
  granted_at      TEXT NOT NULL,
  expires_at      TEXT NOT NULL,
  last_seen_at    TEXT,
  UNIQUE(bucket_id, uri_hash, token_hash)
);

CREATE TABLE IF NOT EXISTS audit_log (
  id          TEXT PRIMARY KEY,
  event_type  TEXT NOT NULL,
  actor       TEXT,
  target_id   TEXT,
  metadata    TEXT,
  created_at  TEXT NOT NULL
);

INSERT OR IGNORE INTO settings (key, value) VALUES
  ('auto_lock_minutes', '30'),
  ('vault_elevation_minutes', '30'),
  ('bucket_elevation_minutes', '15'),
  ('default_access_ttl_minutes', '60'),
  ('default_refresh_ttl_minutes', '0'),
  ('run_in_background', '1'),
  ('vault_read_requires_elevation', '0'),
  ('vault_write_requires_elevation', '1'),
  ('buckets_read_requires_elevation', '0'),
  ('buckets_write_requires_elevation', '1'),
  ('fallback_to_dotenv', '0'),
  ('lock_on_screen_lock', '1'),
  ('notify_client_access', '1'),
  ('expiry_notify_days', '7');
