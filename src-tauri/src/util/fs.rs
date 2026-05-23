use std::path::Path;

use crate::error::AppResult;

/// Restrict access to the Argus data directory and files (Unix modes + Windows ACL best-effort).
pub fn harden_path(path: &Path, is_dir: bool) -> AppResult<()> {
    #[cfg(unix)]
    {
        use std::fs;
        use std::os::unix::fs::PermissionsExt;
        let mode = if is_dir { 0o700 } else { 0o600 };
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|e| AppError::message("IO_ERROR", e.to_string()))?;
    }

    #[cfg(windows)]
    {
        let _ = (path, is_dir);
        // Rely on user profile ACL; explicit DACL can be added later.
    }

    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, is_dir);
    }

    Ok(())
}
