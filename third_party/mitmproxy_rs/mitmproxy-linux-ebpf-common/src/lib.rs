#![no_std]

// aya-ebpf currently does not compile on Windows.
#[cfg(target_os = "linux")]
use aya_ebpf::TASK_COMM_LEN;
#[cfg(not(target_os = "linux"))]
const TASK_COMM_LEN: usize = 16;

type Pid = u32;

pub const INTERCEPT_CONF_LEN: u32 = 20;

/// IPv4 flow key for PID lookup (matches userspace `FlowKey`).
#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct FlowKey {
    pub saddr: u32,
    pub daddr: u32,
    pub sport: u16,
    pub dport: u16,
    pub proto: u8,
    pub _pad: [u8; 3],
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub enum Pattern {
    Pid(Pid),
    Process([u8; TASK_COMM_LEN]),
}

#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub enum Action {
    None,
    Include(Pattern),
    Exclude(Pattern),
}

impl Pattern {
    pub fn matches(&self, command: Option<&[u8; TASK_COMM_LEN]>, pid: Pid) -> bool {
        match self {
            Pattern::Pid(p) => pid == *p,
            Pattern::Process(process) => {
                let Some(command) = command else {
                    return false;
                };
                for i in 0..16 {
                    let curr = command[i];
                    if curr != process[i] {
                        return false;
                    }
                    if curr == 0 {
                        break;
                    }
                }
                true
            }
        }
    }
}

impl From<&str> for Action {
    fn from(value: &str) -> Self {
        let value = value.trim();
        if let Some(value) = value.strip_prefix('!') {
            Action::Exclude(Pattern::from(value))
        } else {
            Action::Include(Pattern::from(value))
        }
    }
}

impl From<&str> for Pattern {
    fn from(value: &str) -> Self {
        let value = value.trim();
        match value.parse::<u32>() {
            Ok(pid) => Pattern::Pid(pid),
            Err(_) => {
                let mut val = [0u8; TASK_COMM_LEN];
                let src = value.as_bytes();
                let len = core::cmp::min(TASK_COMM_LEN - 1, src.len());
                val[..len].copy_from_slice(&src[..len]);
                Pattern::Process(val)
            }
        }
    }
}
