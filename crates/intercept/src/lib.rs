//! Relay OS-captured TCP streams to Argus bucket transparent port.

mod relay;
mod server;

pub use server::{
    elevation_notice, has_capture_privileges, start_linux_redirector, start_windows_redirector,
    unavailable_reason, RedirectorHandle,
};
