//! Windows named-pipe security: current-user DACL for cross-integrity IPC.

#[cfg(windows)]
pub fn create_ipc_pipe(
    addr: &str,
    first_instance: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer, std::io::Error> {
    use std::mem::MaybeUninit;
    use std::ptr::{addr_of_mut, null_mut};
    use tokio::net::windows::named_pipe::ServerOptions;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::Security::{
        AddAccessAllowedAce, GetLengthSid, GetTokenInformation, InitializeAcl,
        InitializeSecurityDescriptor, SetSecurityDescriptorDacl, ACL_REVISION, TokenUser,
        ACCESS_ALLOWED_ACE, ACL, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, SECURITY_DESCRIPTOR,
        TOKEN_QUERY, TOKEN_USER,
    };
    use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    // GENERIC_READ | GENERIC_WRITE | SYNCHRONIZE
    const PIPE_ACCESS: u32 = 0x80000000 | 0x40000000 | 0x00100000;

    unsafe {
        let mut token: HANDLE = null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) == 0 {
            return create_fallback(addr, first_instance);
        }

        let mut needed = 0u32;
        let _ = GetTokenInformation(token, TokenUser, null_mut(), 0, &mut needed);
        if needed == 0 {
            CloseHandle(token);
            return create_fallback(addr, first_instance);
        }

        let mut token_buf = vec![0u8; needed as usize];
        if GetTokenInformation(
            token,
            TokenUser,
            token_buf.as_mut_ptr() as *mut _,
            needed,
            &mut needed,
        ) == 0
        {
            CloseHandle(token);
            return create_fallback(addr, first_instance);
        }
        CloseHandle(token);

        let token_user = &*(token_buf.as_ptr() as *const TOKEN_USER);
        let user_sid = token_user.User.Sid;

        let sid_len = GetLengthSid(user_sid) as usize;
        let acl_size = std::mem::size_of::<ACL>()
            + std::mem::size_of::<ACCESS_ALLOWED_ACE>()
            - std::mem::size_of::<u32>()
            + sid_len;
        let mut acl_buf = vec![0u8; acl_size];
        let acl = acl_buf.as_mut_ptr() as *mut ACL;

        if InitializeAcl(acl, acl_size as u32, ACL_REVISION) == 0 {
            return create_fallback(addr, first_instance);
        }
        if AddAccessAllowedAce(acl, ACL_REVISION, PIPE_ACCESS, user_sid) == 0 {
            return create_fallback(addr, first_instance);
        }

        let mut sec_desc: SECURITY_DESCRIPTOR = MaybeUninit::zeroed().assume_init();
        if InitializeSecurityDescriptor(
            addr_of_mut!(sec_desc) as PSECURITY_DESCRIPTOR,
            1,
        ) == 0
        {
            return create_fallback(addr, first_instance);
        }
        if SetSecurityDescriptorDacl(
            addr_of_mut!(sec_desc) as PSECURITY_DESCRIPTOR,
            1,
            acl,
            0,
        ) == 0
        {
            return create_fallback(addr, first_instance);
        }

        let mut sec_attr = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: addr_of_mut!(sec_desc) as PSECURITY_DESCRIPTOR,
            bInheritHandle: 0,
        };

        let mut opts = ServerOptions::new();
        if first_instance {
            opts.first_pipe_instance(true);
        }
        opts.create_with_security_attributes_raw(addr, addr_of_mut!(sec_attr) as *mut _)
    }
}

#[cfg(windows)]
fn create_fallback(
    addr: &str,
    first_instance: bool,
) -> Result<tokio::net::windows::named_pipe::NamedPipeServer, std::io::Error> {
    let mut opts = tokio::net::windows::named_pipe::ServerOptions::new();
    if first_instance {
        opts.first_pipe_instance(true);
    }
    opts.create(addr)
}
