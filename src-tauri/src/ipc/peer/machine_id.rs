/// Reads a stable machine identifier from the OS.
/// Windows: MachineGuid from registry.
/// Linux: /etc/machine-id.
/// macOS: IOPlatformUUID via ioreg.

pub fn read_machine_id() -> String {
    platform_machine_id().unwrap_or_else(|| "unknown-machine".to_string())
}

#[cfg(target_os = "windows")]
fn platform_machine_id() -> Option<String> {
    use std::process::Command;
    let output = Command::new("reg")
        .args([
            "query",
            r"HKLM\SOFTWARE\Microsoft\Cryptography",
            "/v",
            "MachineGuid",
        ])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if let Some(pos) = line.find("REG_SZ") {
            let value = line[pos + 6..].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

#[cfg(target_os = "linux")]
fn platform_machine_id() -> Option<String> {
    std::fs::read_to_string("/etc/machine-id")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(target_os = "macos")]
fn platform_machine_id() -> Option<String> {
    use std::process::Command;
    let output = Command::new("ioreg")
        .args(["-rd1", "-c", "IOPlatformExpertDevice"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    for line in text.lines() {
        if line.contains("IOPlatformUUID") {
            if let Some(start) = line.find('"') {
                let rest = &line[start + 1..];
                if let Some(end) = rest.rfind('"') {
                    let uuid = &rest[..end];
                    if !uuid.is_empty() {
                        return Some(uuid.to_string());
                    }
                }
            }
        }
    }
    None
}

#[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
fn platform_machine_id() -> Option<String> {
    None
}
