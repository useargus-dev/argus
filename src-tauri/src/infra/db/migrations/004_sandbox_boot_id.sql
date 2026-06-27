-- Process boot identity for sandbox PID registry (PID reuse protection)
ALTER TABLE sandbox_session_pids ADD COLUMN process_boot_id TEXT NOT NULL DEFAULT '';
