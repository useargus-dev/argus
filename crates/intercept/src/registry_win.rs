#[cfg(windows)]
use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};

#[cfg(windows)]
pub fn read_install_path_registry() -> anyhow::Result<std::path::PathBuf> {
    read_install_path_from_hive(HKEY_LOCAL_MACHINE)
        .or_else(|_| read_install_path_from_hive(HKEY_CURRENT_USER))
}

#[cfg(windows)]
fn read_install_path_from_hive(
    hive: windows_sys::Win32::System::Registry::HKEY,
) -> anyhow::Result<std::path::PathBuf> {
    use std::os::windows::ffi::OsStringExt;
    use std::path::PathBuf;
    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegOpenKeyExW, RegQueryValueExW, HKEY, KEY_READ, REG_SZ,
    };

    let subkey: Vec<u16> = "Software\\Argus\0".encode_utf16().collect();
    let value_name: Vec<u16> = "InstallPath\0".encode_utf16().collect();
    unsafe {
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(hive, subkey.as_ptr(), 0, KEY_READ, &mut key) != 0 {
            anyhow::bail!("registry key not found");
        }
        let mut kind = 0u32;
        let mut size = 0u32;
        if RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            std::ptr::null_mut(),
            &mut size,
        ) != 0
            || kind != REG_SZ
        {
            RegCloseKey(key);
            anyhow::bail!("InstallPath not found");
        }
        let mut buf = vec![0u16; (size as usize).max(2) / 2];
        if RegQueryValueExW(
            key,
            value_name.as_ptr(),
            std::ptr::null_mut(),
            &mut kind,
            buf.as_mut_ptr() as *mut u8,
            &mut size,
        ) != 0
        {
            RegCloseKey(key);
            anyhow::bail!("failed to read InstallPath");
        }
        RegCloseKey(key);
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        Ok(PathBuf::from(std::ffi::OsString::from_wide(&buf[..len])))
    }
}
