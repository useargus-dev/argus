use std::path::PathBuf;

pub use intercept::{
    argus_home, linux_redirector_path, redirector_dir, windows_redirector_path,
};

pub fn redirector_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        return linux_redirector_path();
    }
    #[cfg(windows)]
    {
        return windows_redirector_path();
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        redirector_dir()
    }
}
