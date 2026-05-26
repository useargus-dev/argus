//! Platform-native process introspection (command line & cwd).
//!
//! Bypasses `sysinfo` which often returns empty on Windows.

/// Process details retrieved via OS-native APIs.
pub struct ProcInfo {
    pub cmd_line: String,
    pub cwd: Option<String>,
}

#[cfg(windows)]
pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
    win::read_proc_info(pid)
}

#[cfg(target_os = "linux")]
pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
    linux::read_proc_info(pid)
}

#[cfg(target_os = "macos")]
pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
    macos::read_proc_info(pid)
}

#[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
pub fn read_proc_info(_pid: u32) -> Option<ProcInfo> {
    None
}

// ---------------------------------------------------------------------------
// Windows: read command line via NtQueryInformationProcess + PEB
// ---------------------------------------------------------------------------
#[cfg(windows)]
mod win {
    use super::ProcInfo;
    use std::mem;
    use std::ptr;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, UNICODE_STRING};
    use windows_sys::Win32::System::Threading::{
        OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
    };

    const PROCESS_BASIC_INFORMATION_CLASS: u32 = 0;

    #[repr(C)]
    struct ProcessBasicInformation {
        reserved1: usize,
        peb_base_address: usize,
        reserved2: [usize; 2],
        unique_process_id: usize,
        reserved3: usize,
    }

    #[repr(C)]
    struct PebPartial {
        reserved: [u8; 0x20],
        process_parameters: usize,
    }

    #[repr(C)]
    struct RtlUserProcessParametersPartial {
        reserved: [u8; 0x38],
        current_directory_path: UNICODE_STRING,
        _pad: [u8; 8],
        _dll_path: UNICODE_STRING,
        _image_path: UNICODE_STRING,
        command_line: UNICODE_STRING,
    }

    extern "system" {
        fn NtQueryInformationProcess(
            process_handle: HANDLE,
            process_information_class: u32,
            process_information: *mut u8,
            process_information_length: u32,
            return_length: *mut u32,
        ) -> i32;

        fn ReadProcessMemory(
            h_process: HANDLE,
            lp_base_address: usize,
            lp_buffer: *mut u8,
            n_size: usize,
            lp_number_of_bytes_read: *mut usize,
        ) -> i32;
    }

    pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
        unsafe {
            let handle = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ,
                0,
                pid,
            );
            if handle.is_null() {
                return None;
            }
            let result = read_from_handle(handle);
            CloseHandle(handle);
            result
        }
    }

    unsafe fn read_from_handle(handle: HANDLE) -> Option<ProcInfo> {
        let mut pbi: ProcessBasicInformation = mem::zeroed();
        let mut ret_len: u32 = 0;
        let status = NtQueryInformationProcess(
            handle,
            PROCESS_BASIC_INFORMATION_CLASS,
            &mut pbi as *mut _ as *mut u8,
            mem::size_of::<ProcessBasicInformation>() as u32,
            &mut ret_len,
        );
        if status != 0 || pbi.peb_base_address == 0 {
            return None;
        }

        let mut peb: PebPartial = mem::zeroed();
        if !read_mem(handle, pbi.peb_base_address, &mut peb) {
            return None;
        }
        if peb.process_parameters == 0 {
            return None;
        }

        let mut params: RtlUserProcessParametersPartial = mem::zeroed();
        if !read_mem(handle, peb.process_parameters, &mut params) {
            return None;
        }

        let cmd_line = read_unicode_string(handle, &params.command_line)?;
        let cwd = read_unicode_string(handle, &params.current_directory_path);

        Some(ProcInfo { cmd_line, cwd })
    }

    unsafe fn read_mem<T>(handle: HANDLE, addr: usize, out: &mut T) -> bool {
        let mut read: usize = 0;
        let ok = ReadProcessMemory(
            handle,
            addr,
            out as *mut T as *mut u8,
            mem::size_of::<T>(),
            &mut read,
        );
        ok != 0 && read == mem::size_of::<T>()
    }

    unsafe fn read_unicode_string(handle: HANDLE, us: &UNICODE_STRING) -> Option<String> {
        let len = us.Length as usize;
        if len == 0 || us.Buffer == ptr::null_mut() {
            return None;
        }
        let mut buf: Vec<u16> = vec![0u16; len / 2];
        let mut read: usize = 0;
        let ok = ReadProcessMemory(
            handle,
            us.Buffer as usize,
            buf.as_mut_ptr() as *mut u8,
            len,
            &mut read,
        );
        if ok == 0 || read < len {
            return None;
        }
        let s = String::from_utf16_lossy(&buf);
        let trimmed = s.trim_end_matches('\\').trim().to_string();
        if trimmed.is_empty() { None } else { Some(trimmed) }
    }
}

// ---------------------------------------------------------------------------
// Linux: read from /proc filesystem
// ---------------------------------------------------------------------------
#[cfg(target_os = "linux")]
mod linux {
    use super::ProcInfo;
    use std::fs;

    pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
        let cmdline_path = format!("/proc/{pid}/cmdline");
        let raw = fs::read(&cmdline_path).ok()?;
        if raw.is_empty() {
            return None;
        }
        let cmd_line = raw
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect::<Vec<_>>()
            .join(" ");

        let cwd_link = format!("/proc/{pid}/cwd");
        let cwd = fs::read_link(&cwd_link)
            .ok()
            .and_then(|p| p.to_str().map(String::from));

        Some(ProcInfo {
            cmd_line,
            cwd,
        })
    }
}

// ---------------------------------------------------------------------------
// macOS: use sysctl KERN_PROCARGS2 for command line, libproc for cwd
// ---------------------------------------------------------------------------
#[cfg(target_os = "macos")]
mod macos {
    use super::ProcInfo;
    use std::mem;
    use std::ptr;

    pub fn read_proc_info(pid: u32) -> Option<ProcInfo> {
        let cmd_line = read_cmdline(pid as i32)?;
        let cwd = read_cwd(pid as i32);
        Some(ProcInfo { cmd_line, cwd })
    }

    fn read_cmdline(pid: i32) -> Option<String> {
        let mut mib: [i32; 3] = [libc::CTL_KERN, libc::KERN_PROCARGS2, pid];
        let mut size: libc::size_t = 0;

        // First call to get buffer size
        let ret = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                3,
                ptr::null_mut(),
                &mut size,
                ptr::null_mut(),
                0,
            )
        };
        if ret != 0 || size == 0 {
            return None;
        }

        let mut buf: Vec<u8> = vec![0; size];
        let ret = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                3,
                buf.as_mut_ptr() as *mut _,
                &mut size,
                ptr::null_mut(),
                0,
            )
        };
        if ret != 0 {
            return None;
        }
        buf.truncate(size);

        if buf.len() < mem::size_of::<i32>() {
            return None;
        }

        // First 4 bytes = argc
        let argc = i32::from_ne_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
        let mut pos = mem::size_of::<i32>();

        // Skip the exec path (null-terminated)
        while pos < buf.len() && buf[pos] != 0 {
            pos += 1;
        }
        // Skip null bytes between exec path and argv[0]
        while pos < buf.len() && buf[pos] == 0 {
            pos += 1;
        }

        // Read argc arguments
        let mut args = Vec::with_capacity(argc);
        for _ in 0..argc {
            if pos >= buf.len() {
                break;
            }
            let start = pos;
            while pos < buf.len() && buf[pos] != 0 {
                pos += 1;
            }
            args.push(String::from_utf8_lossy(&buf[start..pos]).into_owned());
            pos += 1; // skip null
        }

        if args.is_empty() {
            None
        } else {
            Some(args.join(" "))
        }
    }

    fn read_cwd(pid: i32) -> Option<String> {
        // PROC_PIDVNODEPATHINFO = 9
        const PROC_PIDVNODEPATHINFO: i32 = 9;
        const MAXPATHLEN: usize = 1024;

        #[repr(C)]
        struct VnodeInfoPath {
            _vip_vi: [u8; 152], // struct vnode_info
            vip_path: [u8; MAXPATHLEN],
        }

        #[repr(C)]
        struct ProcVnodePathInfo {
            pvi_cdir: VnodeInfoPath,
            _pvi_rdir: VnodeInfoPath,
        }

        extern "C" {
            fn proc_pidinfo(
                pid: i32,
                flavor: i32,
                arg: u64,
                buffer: *mut u8,
                buffersize: i32,
            ) -> i32;
        }

        let mut info: ProcVnodePathInfo = unsafe { mem::zeroed() };
        let size = mem::size_of::<ProcVnodePathInfo>() as i32;
        let ret = unsafe {
            proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, 0, &mut info as *mut _ as *mut u8, size)
        };
        if ret <= 0 {
            return None;
        }

        let path_bytes = &info.pvi_cdir.vip_path;
        let len = path_bytes.iter().position(|&b| b == 0).unwrap_or(MAXPATHLEN);
        let s = String::from_utf8_lossy(&path_bytes[..len]).into_owned();
        if s.is_empty() { None } else { Some(s) }
    }
}
