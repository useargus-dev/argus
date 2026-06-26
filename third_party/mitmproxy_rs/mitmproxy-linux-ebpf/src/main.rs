#![no_std]
#![no_main]

use aya_ebpf::EbpfContext;
use aya_ebpf::macros::{cgroup_connect4, cgroup_sock, map};
use aya_ebpf::maps::{Array, LruHashMap};
use aya_ebpf::programs::{CgroupSockContext, SockContext};
use aya_log_ebpf::debug;
use mitmproxy_linux_ebpf_common::{Action, FlowKey, INTERCEPT_CONF_LEN};

#[unsafe(no_mangle)]
static INTERFACE_ID: u32 = 0;

#[map]
static INTERCEPT_CONF: Array<Action> = Array::with_max_entries(INTERCEPT_CONF_LEN, 0);

#[map]
static FLOW_PID: LruHashMap<FlowKey, u32> = LruHashMap::with_max_entries(8192, 0);

#[cgroup_sock(sock_create)]
pub fn cgroup_sock_create(ctx: SockContext) -> i32 {
    if should_intercept(&ctx) {
        debug!(&ctx, "intercepting in sock_create");
        let interface_id = unsafe { core::ptr::read_volatile(&INTERFACE_ID) };
        unsafe {
            (*ctx.sock).bound_dev_if = interface_id;
        }
    }
    1
}

#[cgroup_connect4]
pub fn cgroup_connect4(ctx: SockContext) -> i32 {
    if !should_intercept(&ctx) {
        return 1;
    }
    let pid = ctx.pid();
    let key = flow_key_from_sock(&ctx);
    let _ = unsafe { FLOW_PID.insert(&key, &pid, 0) };
    1
}

fn flow_key_from_sock(ctx: &SockContext) -> FlowKey {
    let saddr = unsafe { (*ctx.sock).src_ip4 };
    let daddr = unsafe { (*ctx.sock).dst_ip4 };
    let sport = unsafe { (*ctx.sock).src_port };
    let dport = unsafe { (*ctx.sock).dst_port };
    FlowKey {
        saddr,
        daddr,
        sport,
        dport,
        proto: 6,
        _pad: [0; 3],
    }
}

pub fn should_intercept(ctx: &SockContext) -> bool {
    let command = ctx.command().ok();
    let pid = ctx.pid();

    let mut intercept = matches!(INTERCEPT_CONF.get(0), Some(Action::Exclude(_)));
    for i in 0..INTERCEPT_CONF_LEN {
        match INTERCEPT_CONF.get(i) {
            Some(Action::Include(pattern)) => {
                intercept = intercept || pattern.matches(command.as_ref(), pid);
            }
            Some(Action::Exclude(pattern)) => {
                intercept = intercept && !pattern.matches(command.as_ref(), pid);
            }
            _ => {
                break;
            }
        }
    }
    intercept
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
