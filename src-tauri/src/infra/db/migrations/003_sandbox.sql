-- Sandbox sessions for argus run (transparent capture via PID registry)
CREATE TABLE sandbox_sessions (
  id TEXT PRIMARY KEY,
  bucket_id TEXT NOT NULL,
  grant_id TEXT NOT NULL,
  parent_fingerprint TEXT NOT NULL,
  command_preview TEXT,
  root_pid INTEGER,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  revoked_at TEXT,
  FOREIGN KEY (bucket_id) REFERENCES app_buckets(id)
);

CREATE TABLE sandbox_session_pids (
  session_id TEXT NOT NULL,
  pid INTEGER NOT NULL,
  added_at TEXT NOT NULL,
  PRIMARY KEY (session_id, pid),
  FOREIGN KEY (session_id) REFERENCES sandbox_sessions(id)
);

CREATE INDEX idx_sandbox_session_pids_pid ON sandbox_session_pids(pid);
