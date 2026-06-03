use crate::error::{AppError, AppResult};

#[cfg(unix)]
pub fn peer_pid_from_stream<S>(stream: &S) -> AppResult<u32>
where
    S: std::os::unix::io::AsRawFd,
{
    #[cfg(target_os = "linux")]
    {
        use nix::sys::socket::getsockopt;
        use nix::sys::socket::sockopt::PeerCredentials;
        use std::os::unix::io::{AsRawFd, BorrowedFd};

        let fd = stream.as_raw_fd();
        let borrowed = unsafe { BorrowedFd::borrow_raw(fd) };
        let cred = getsockopt(&borrowed, PeerCredentials)
            .map_err(|e| AppError::message("PEER_RESOLVE", e.to_string()))?;
        let pid = cred.pid();
        if pid <= 0 {
            return Err(AppError::message("PEER_RESOLVE", "invalid peer pid"));
        }
        return Ok(pid as u32);
    }

    #[cfg(target_os = "macos")]
    {
        use std::os::raw::c_int;
        use std::os::unix::io::AsRawFd;

        let fd = stream.as_raw_fd();
        const LOCAL_PEERPID: c_int = 0x002;
        let mut pid: u32 = 0;
        let r = unsafe {
            libc::getsockopt(
                fd,
                libc::SOL_LOCAL,
                LOCAL_PEERPID,
                &mut pid as *mut _ as *mut _,
                &mut (std::mem::size_of::<u32>() as u32) as *mut _,
            )
        };
        if r != 0 || pid == 0 {
            return Err(AppError::message("PEER_RESOLVE", "could not read peer pid"));
        }
        return Ok(pid);
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = stream;
        Err(AppError::message(
            "PEER_RESOLVE",
            "TCP peer pid not supported on this unix platform",
        ))
    }
}

#[cfg(windows)]
pub fn peer_pid_from_stream<S>(stream: &S) -> AppResult<u32>
where
    S: peer_pid_from_tcp::PeerAddrs,
{
    let local = stream
        .local_addr()
        .map_err(|e| AppError::message("PEER_RESOLVE", e.to_string()))?;
    let remote = stream
        .peer_addr()
        .map_err(|e| AppError::message("PEER_RESOLVE", e.to_string()))?;
    // On an accepted server socket, local=proxy listener and remote=client ephemeral.
    // GetExtendedTcpTable rows are per owning process; the client-side row has swapped endpoints.
    peer_pid_from_tcp::lookup_owner_pid(remote, local)
}

#[cfg(not(any(unix, windows)))]
pub fn peer_pid_from_stream<S>(_stream: &S) -> AppResult<u32> {
    Err(AppError::message("PEER_RESOLVE", "unsupported platform"))
}

#[cfg(windows)]
mod peer_pid_from_tcp {
    use std::mem;
    use std::net::{Ipv4Addr, SocketAddr};

    use windows_sys::Win32::Foundation::{ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows_sys::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, MIB_TCPTABLE_OWNER_PID,
        TCP_TABLE_OWNER_PID_ALL,
    };

    use crate::error::{AppError, AppResult};

    pub trait PeerAddrs {
        fn local_addr(&self) -> std::io::Result<SocketAddr>;
        fn peer_addr(&self) -> std::io::Result<SocketAddr>;
    }

    impl PeerAddrs for tokio::net::TcpStream {
        fn local_addr(&self) -> std::io::Result<SocketAddr> {
            self.local_addr()
        }

        fn peer_addr(&self) -> std::io::Result<SocketAddr> {
            self.peer_addr()
        }
    }

    fn ipv4_to_dw(ip: Ipv4Addr) -> u32 {
        u32::from_le_bytes(ip.octets())
    }

    /// Windows `MIB_TCPROW_OWNER_PID` stores ports in network byte order (see `ntohs`).
    fn port_to_dw(port: u16) -> u32 {
        u32::from(port.swap_bytes())
    }

    pub fn lookup_owner_pid(local: SocketAddr, remote: SocketAddr) -> AppResult<u32> {
        let (local_addr, local_port) = match local {
            SocketAddr::V4(v4) => (ipv4_to_dw(*v4.ip()), port_to_dw(v4.port())),
            _ => {
                return Err(AppError::message(
                    "PEER_RESOLVE",
                    "IPv6 proxy peer lookup is not supported",
                ))
            }
        };
        let (remote_addr, remote_port) = match remote {
            SocketAddr::V4(v4) => (ipv4_to_dw(*v4.ip()), port_to_dw(v4.port())),
            _ => {
                return Err(AppError::message(
                    "PEER_RESOLVE",
                    "IPv6 proxy peer lookup is not supported",
                ))
            }
        };

        let mut size: u32 = 0;
        let ret = unsafe {
            GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                windows_sys::Win32::Networking::WinSock::AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if ret != ERROR_INSUFFICIENT_BUFFER && ret != NO_ERROR {
            return Err(AppError::message(
                "PEER_RESOLVE",
                format!("GetExtendedTcpTable size query failed: {ret}"),
            ));
        }

        let mut buf = vec![0u8; size as usize];
        let ret = unsafe {
            GetExtendedTcpTable(
                buf.as_mut_ptr().cast(),
                &mut size,
                0,
                windows_sys::Win32::Networking::WinSock::AF_INET as u32,
                TCP_TABLE_OWNER_PID_ALL,
                0,
            )
        };
        if ret != NO_ERROR {
            return Err(AppError::message(
                "PEER_RESOLVE",
                format!("GetExtendedTcpTable failed: {ret}"),
            ));
        }

        let table = unsafe { &*(buf.as_ptr() as *const MIB_TCPTABLE_OWNER_PID) };
        let row_count = table.dwNumEntries as usize;
        let row_size = mem::size_of::<MIB_TCPROW_OWNER_PID>();
        let base = unsafe { buf.as_ptr().add(mem::size_of::<MIB_TCPTABLE_OWNER_PID>()) };

        for i in 0..row_count {
            let row = unsafe { &*(base.add(i * row_size) as *const MIB_TCPROW_OWNER_PID) };
            if row.dwLocalAddr == local_addr
                && row.dwLocalPort == local_port
                && row.dwRemoteAddr == remote_addr
                && row.dwRemotePort == remote_port
                && row.dwOwningPid != 0
            {
                return Ok(row.dwOwningPid);
            }
        }

        Err(AppError::message(
            "PEER_RESOLVE",
            "could not resolve TCP connection owner pid",
        ))
    }
}
