//! Parse IPv4 headers from TUN packets for FLOW_PID lookup.

use mitmproxy_linux_ebpf_common::FlowKey;

const IPV4_PROTO: u8 = 4;
const TCP_PROTO: u8 = 6;
const UDP_PROTO: u8 = 17;

/// Extract IPv4 flow key from a raw L3 packet (may include Ethernet — caller should pass IP start).
pub fn flow_key_from_ipv4_packet(data: &[u8]) -> Option<FlowKey> {
    if data.len() < 20 {
        return None;
    }
    let version = data[0] >> 4;
    if version != IPV4_PROTO {
        return None;
    }
    let ihl = (data[0] & 0x0f) as usize * 4;
    if data.len() < ihl + 4 {
        return None;
    }
    let proto = data[9];
    if proto != TCP_PROTO && proto != UDP_PROTO {
        return None;
    }
    let saddr = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
    let daddr = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
    let l4 = &data[ihl..];
    if l4.len() < 4 {
        return None;
    }
    let sport = u16::from_be_bytes([l4[0], l4[1]]);
    let dport = u16::from_be_bytes([l4[2], l4[3]]);
    Some(FlowKey {
        saddr,
        daddr,
        sport,
        dport,
        proto,
        _pad: [0; 3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_tcp_ipv4() {
        let mut pkt = vec![0u8; 40];
        pkt[0] = 0x45;
        pkt[9] = TCP_PROTO;
        pkt[12..16].copy_from_slice(&[10, 0, 0, 1]);
        pkt[16..20].copy_from_slice(&[93, 184, 216, 34]);
        pkt[20..22].copy_from_slice(&[0xC0, 0x00]);
        pkt[22..24].copy_from_slice(&[0x01, 0xBB]);
        let key = flow_key_from_ipv4_packet(&pkt).unwrap();
        assert_eq!(key.dport, 443);
        assert_eq!(key.proto, TCP_PROTO);
    }
}
