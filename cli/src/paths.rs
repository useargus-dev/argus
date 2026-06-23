use std::path::PathBuf;

pub fn argus_home() -> PathBuf {
    redirector_core::argus_home()
}

pub fn redirector_path() -> PathBuf {
    #[cfg(target_os = "linux")]
    {
        return redirector_core::linux_redirector_path();
    }
    #[cfg(windows)]
    {
        return redirector_core::windows_redirector_path();
    }
    #[cfg(not(any(target_os = "linux", windows)))]
    {
        redirector_core::redirector_dir()
    }
}
