#![no_std]
#![no_main]

use aya_ebpf::EbpfContext;
use aya_ebpf::macros::{cgroup_sock, cgroup_sock_addr, map};
use aya_ebpf::maps::{Array, LruHashMap};
use aya_ebpf::programs::{SockAddrContext, SockContext};
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

#[cgroup_sock_addr(connect4)]
pub fn cgroup_connect4(ctx: SockAddrContext) -> i32 {
    if !should_intercept_connect(&ctx) {
        return 1;
    }
    let pid = ctx.pid();
    let key = flow_key_from_connect4(&ctx);
    let _ = FLOW_PID.insert(&key, &pid, 0);
    1
}

fn flow_key_from_connect4(ctx: &SockAddrContext) -> FlowKey {
    let addr = unsafe { &*ctx.sock_addr };
    let sk = unsafe { addr.__bindgen_anon_1.sk };
    if !sk.is_null() {
        let sock = unsafe { &*sk };
        return FlowKey {
            saddr: sock.src_ip4,
            daddr: sock.dst_ip4,
            sport: sock.src_port as u16,
            dport: u16::from_be(sock.dst_port),
            proto: 6,
            _pad: [0; 3],
        };
    }
    FlowKey {
        saddr: addr.msg_src_ip4,
        daddr: addr.user_ip4,
        sport: 0,
        dport: addr.user_port as u16,
        proto: 6,
        _pad: [0; 3],
    }
}

fn should_intercept(ctx: &SockContext) -> bool {
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

fn should_intercept_connect(ctx: &SockAddrContext) -> bool {
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
