//! Relay OS-captured TCP streams to Argus bucket transparent port.

mod paths;
mod relay;
#[cfg(windows)]
mod registry_win;
mod server;

pub use paths::{
    argus_home, intercept_spec_for_pid, intercept_spec_for_pids, linux_redirector_path,
    redirector_dir, start_redirector, windows_redirector_path,
};
pub use server::{
    elevation_notice, has_capture_privileges, start_linux_redirector, start_windows_redirector,
    unavailable_reason, RedirectorHandle,
};
