-- Migration 003: Store process details in client_grants for audit display.

ALTER TABLE client_grants ADD COLUMN cwd TEXT;
ALTER TABLE client_grants ADD COLUMN exe_path TEXT;
ALTER TABLE client_grants ADD COLUMN git_remote TEXT;
ALTER TABLE client_grants ADD COLUMN run_args TEXT;
