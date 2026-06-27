use std::path::PathBuf;

pub use intercept::argus_home;

#[cfg(target_os = "linux")]
pub use intercept::linux_redirector_path;
#[cfg(windows)]
pub use intercept::{redirector_dir, windows_redirector_path};

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
