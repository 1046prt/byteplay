use pnet::packet::ip::IpNextHeaderProtocol;
use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    ipv6::Ipv6Packet,
    tcp::TcpPacket,
    udp::UdpPacket,
    Packet,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParsedPacket {
    pub id: String,
    pub timestamp: String,
    pub interface: String,
    pub frame_length: usize,
    pub ethernet: Option<EthernetInfo>,
    pub ipv4: Option<IPv4Info>,
    pub ipv6: Option<IPv6Info>,
    pub tcp: Option<TcpInfo>,
    pub udp: Option<UdpInfo>,
    pub raw_bytes: Vec<u8>,
    pub payload: Vec<u8>,
    pub payload_hex: String,
    pub payload_ascii: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthernetInfo {
    pub src_mac: String,
    pub dst_mac: String,
    pub ether_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv4Info {
    pub src_ip: String,
    pub dst_ip: String,
    pub version: u8,
    pub ihl: u8,
    pub dscp: u8,
    pub ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags: u8,
    pub fragment_offset: u16,
    pub ttl: u8,
    pub protocol: String,
    pub checksum: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IPv6Info {
    pub src_ip: String,
    pub dst_ip: String,
    pub version: u8,
    pub traffic_class: u8,
    pub flow_label: u32,
    pub payload_length: u16,
    pub next_header: String,
    pub hop_limit: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpInfo {
    pub src_port: u16,
    pub dst_port: u16,
    pub sequence: u32,
    pub ack_number: u32,
    pub data_offset: u8,
    pub flags: TcpFlags,
    pub window: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TcpFlags {
    pub syn: bool,
    pub ack: bool,
    pub fin: bool,
    pub rst: bool,
    pub psh: bool,
    pub urg: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UdpInfo {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

pub fn parse_ethernet(packet: &[u8]) -> Option<EthernetPacket<'_>> {
    EthernetPacket::new(packet)
}

pub const LINKTYPE_ETHERNET: u32 = 1;
pub const LINKTYPE_RAW: u32 = 101;

pub fn parse_packet(raw_bytes: Vec<u8>, interface: String) -> ParsedPacket {
    parse_packet_with_linktype(raw_bytes, interface, LINKTYPE_ETHERNET)
}

pub fn parse_packet_with_linktype(
    raw_bytes: Vec<u8>,
    interface: String,
    linktype: u32,
) -> ParsedPacket {
    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let frame_length = raw_bytes.len();

    let mut result = ParsedPacket {
        id,
        timestamp,
        interface,
        frame_length,
        ethernet: None,
        ipv4: None,
        ipv6: None,
        tcp: None,
        udp: None,
        payload: Vec::new(),
        payload_hex: String::new(),
        payload_ascii: String::new(),
        raw_bytes: raw_bytes.clone(),
    };

    if linktype == LINKTYPE_RAW {
        if let Some(ip) = parse_raw_ip(&raw_bytes) {
            parse_ip_payload(ip, &mut result);
        } else {
            result.payload = raw_bytes;
        }
    } else if linktype == LINKTYPE_ETHERNET {
        if let Some(eth) = parse_ethernet(&raw_bytes) {
            result.ethernet = Some(EthernetInfo {
                src_mac: format_mac(eth.get_source()),
                dst_mac: format_mac(eth.get_destination()),
                ether_type: format!("{:?}", eth.get_ethertype()),
            });

            match eth.get_ethertype() {
                EtherTypes::Ipv4 => {
                    parse_ip_payload(eth.payload(), &mut result);
                }
                EtherTypes::Ipv6 => {
                    parse_ip_payload(eth.payload(), &mut result);
                }
                _ => {
                    result.payload = raw_bytes;
                }
            }
        } else {
            result.payload = raw_bytes;
        }
    }

    result.payload_hex = bytes_to_hex(&result.payload);
    result.payload_ascii = bytes_to_ascii(&result.payload);

    result
}

fn parse_raw_ip(raw: &[u8]) -> Option<&[u8]> {
    match raw.first()? >> 4 {
        4 if raw.len() >= 20 => Some(raw),
        6 if raw.len() >= 40 => Some(raw),
        _ => None,
    }
}

fn parse_ip_payload(payload: &[u8], result: &mut ParsedPacket) {
    match payload.first().map(|b| b >> 4) {
        Some(4) => parse_ipv4(payload, result),
        Some(6) => parse_ipv6(payload, result),
        _ => {}
    }
}

fn parse_ipv4(payload: &[u8], result: &mut ParsedPacket) {
    if let Some(ipv4) = Ipv4Packet::new(payload) {
        let protocol = format_ip_protocol(ipv4.get_next_level_protocol());
        result.ipv4 = Some(IPv4Info {
            src_ip: ipv4.get_source().to_string(),
            dst_ip: ipv4.get_destination().to_string(),
            version: ipv4.get_version(),
            ihl: ipv4.get_header_length(),
            dscp: ipv4.get_dscp(),
            ecn: ipv4.get_ecn(),
            total_length: ipv4.get_total_length(),
            identification: ipv4.get_identification(),
            flags: ipv4.get_flags(),
            fragment_offset: ipv4.get_fragment_offset(),
            ttl: ipv4.get_ttl(),
            protocol: protocol.clone(),
            checksum: ipv4.get_checksum(),
        });

        match ipv4.get_next_level_protocol() {
            IpNextHeaderProtocols::Tcp => {
                if let Some(tcp) = TcpPacket::new(ipv4.payload()) {
                    result.tcp = Some(parse_tcp(&tcp));
                    result.payload = tcp.payload().to_vec();
                }
            }
            IpNextHeaderProtocols::Udp => {
                if let Some(udp) = UdpPacket::new(ipv4.payload()) {
                    result.udp = Some(UdpInfo {
                        src_port: udp.get_source(),
                        dst_port: udp.get_destination(),
                        length: udp.get_length(),
                        checksum: udp.get_checksum(),
                    });
                    result.payload = udp.payload().to_vec();
                }
            }
            _ => {
                result.payload = ipv4.payload().to_vec();
            }
        }
    }
}

fn parse_ipv6(payload: &[u8], result: &mut ParsedPacket) {
    if let Some(ipv6) = Ipv6Packet::new(payload) {
        let protocol = format_ip_protocol(ipv6.get_next_header());
        result.ipv6 = Some(IPv6Info {
            src_ip: ipv6.get_source().to_string(),
            dst_ip: ipv6.get_destination().to_string(),
            version: ipv6.get_version(),
            traffic_class: ipv6.get_traffic_class(),
            flow_label: ipv6.get_flow_label(),
            payload_length: ipv6.get_payload_length(),
            next_header: protocol,
            hop_limit: ipv6.get_hop_limit(),
        });

        match ipv6.get_next_header() {
            IpNextHeaderProtocols::Tcp => {
                if let Some(tcp) = TcpPacket::new(ipv6.payload()) {
                    result.tcp = Some(parse_tcp(&tcp));
                    result.payload = tcp.payload().to_vec();
                }
            }
            IpNextHeaderProtocols::Udp => {
                if let Some(udp) = UdpPacket::new(ipv6.payload()) {
                    result.udp = Some(UdpInfo {
                        src_port: udp.get_source(),
                        dst_port: udp.get_destination(),
                        length: udp.get_length(),
                        checksum: udp.get_checksum(),
                    });
                    result.payload = udp.payload().to_vec();
                }
            }
            _ => {
                result.payload = ipv6.payload().to_vec();
            }
        }
    }
}

fn parse_tcp(tcp: &TcpPacket<'_>) -> TcpInfo {
    let flags_raw = tcp.get_flags();
    TcpInfo {
        src_port: tcp.get_source(),
        dst_port: tcp.get_destination(),
        sequence: tcp.get_sequence(),
        ack_number: tcp.get_acknowledgement(),
        data_offset: tcp.get_data_offset(),
        flags: TcpFlags {
            syn: (flags_raw & 0x02) != 0,
            ack: (flags_raw & 0x10) != 0,
            fin: (flags_raw & 0x01) != 0,
            rst: (flags_raw & 0x04) != 0,
            psh: (flags_raw & 0x08) != 0,
            urg: (flags_raw & 0x20) != 0,
        },
        window: tcp.get_window(),
        checksum: tcp.get_checksum(),
        urgent_pointer: tcp.get_urgent_ptr(),
    }
}

fn format_mac(mac: pnet::util::MacAddr) -> String {
    format!(
        "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
        mac.octets()[0],
        mac.octets()[1],
        mac.octets()[2],
        mac.octets()[3],
        mac.octets()[4],
        mac.octets()[5],
    )
}

fn format_ip_protocol(proto: IpNextHeaderProtocol) -> String {
    match proto {
        IpNextHeaderProtocols::Tcp => "TCP".to_string(),
        IpNextHeaderProtocols::Udp => "UDP".to_string(),
        IpNextHeaderProtocols::Icmp => "ICMP".to_string(),
        IpNextHeaderProtocols::Icmpv6 => "ICMPv6".to_string(),
        IpNextHeaderProtocols::Gre => "GRE".to_string(),
        IpNextHeaderProtocols::Esp => "ESP".to_string(),
        IpNextHeaderProtocols::Ah => "AH".to_string(),
        _ => format!("{:?}", proto),
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn bytes_to_ascii(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (0x20..=0x7e).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

#[allow(dead_code)]
pub fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let cleaned: String = hex.chars().filter(|c| !c.is_whitespace()).collect();
    if cleaned.len() % 2 != 0 {
        return Err("Hex string must have even length".to_string());
    }
    (0..cleaned.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&cleaned[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex at position {}: {}", i, e))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ipv4_segment(src: [u8; 4], dst: [u8; 4], proto: u8, transport: &[u8]) -> Vec<u8> {
        let mut ip = Vec::new();
        ip.push(0x45);
        ip.push(0);
        ip.extend_from_slice(&((20 + transport.len()) as u16).to_be_bytes());
        ip.extend_from_slice(&[0x00, 0x01]);
        ip.extend_from_slice(&0x4000u16.to_be_bytes());
        ip.push(64);
        ip.push(proto);
        ip.extend_from_slice(&[0x00, 0x00]);
        ip.extend_from_slice(&src);
        ip.extend_from_slice(&dst);
        ip.extend_from_slice(transport);
        ip
    }

    fn tcp_segment(sport: u16, dport: u16, payload: &[u8]) -> Vec<u8> {
        let mut tcp = Vec::new();
        tcp.extend_from_slice(&sport.to_be_bytes());
        tcp.extend_from_slice(&dport.to_be_bytes());
        tcp.extend_from_slice(&1000u32.to_be_bytes());
        tcp.extend_from_slice(&2000u32.to_be_bytes());
        tcp.push(0x50);
        tcp.push(0x18);
        tcp.extend_from_slice(&65535u16.to_be_bytes());
        tcp.extend_from_slice(&[0x00, 0x00]);
        tcp.extend_from_slice(&[0x00, 0x00]);
        tcp.extend_from_slice(payload);
        tcp
    }

    fn udp_segment(sport: u16, dport: u16, payload: &[u8]) -> Vec<u8> {
        let mut udp = Vec::new();
        udp.extend_from_slice(&sport.to_be_bytes());
        udp.extend_from_slice(&dport.to_be_bytes());
        udp.extend_from_slice(&((8 + payload.len()) as u16).to_be_bytes());
        udp.extend_from_slice(&[0x00, 0x00]);
        udp.extend_from_slice(payload);
        udp
    }

    fn eth_frame(ethertype: [u8; 2], payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::new();
        frame.extend_from_slice(&[0x00; 6]);
        frame.extend_from_slice(&[0x00; 6]);
        frame.extend_from_slice(&ethertype);
        frame.extend_from_slice(payload);
        frame
    }

    fn tcp_frame(payload: &[u8]) -> Vec<u8> {
        eth_frame(
            [0x08, 0x00],
            &ipv4_segment(
                [192, 168, 1, 10],
                [192, 168, 1, 20],
                6,
                &tcp_segment(12345, 80, payload),
            ),
        )
    }

    #[test]
    fn parses_ethernet_ipv4_tcp_frame() {
        let frame = tcp_frame(b"GET / HTTP/1.1");
        let parsed = parse_packet(frame.clone(), "eth0".into());

        let eth = parsed.ethernet.expect("ethernet");
        assert_eq!(eth.src_mac, "00:00:00:00:00:00");

        let ip = parsed.ipv4.expect("ipv4");
        assert_eq!(ip.src_ip, "192.168.1.10");
        assert_eq!(ip.dst_ip, "192.168.1.20");
        assert_eq!(ip.protocol, "TCP");
        assert_eq!(ip.version, 4);
        assert_eq!(ip.ttl, 64);

        let tcp = parsed.tcp.expect("tcp");
        assert_eq!(tcp.src_port, 12345);
        assert_eq!(tcp.dst_port, 80);
        assert_eq!(tcp.sequence, 1000);
        assert_eq!(tcp.ack_number, 2000);
        assert!(tcp.flags.psh);
        assert!(tcp.flags.ack);
        assert!(!tcp.flags.syn);

        assert_eq!(parsed.payload, b"GET / HTTP/1.1");
        assert_eq!(parsed.frame_length, frame.len());
    }

    #[test]
    fn parses_udp_frame() {
        let frame = eth_frame(
            [0x08, 0x00],
            &ipv4_segment(
                [10, 0, 0, 1],
                [10, 0, 0, 2],
                17,
                &udp_segment(5353, 5353, b"mdns"),
            ),
        );
        let parsed = parse_packet(frame, "eth0".into());
        let udp = parsed.udp.expect("udp");
        assert_eq!(udp.src_port, 5353);
        assert_eq!(udp.dst_port, 5353);
        assert_eq!(parsed.payload, b"mdns");
        assert!(parsed.tcp.is_none());
    }

    #[test]
    fn truncated_tcp_header_yields_no_transport() {
        let frame = tcp_frame(b"hello");
        let cut: Vec<u8> = frame[..14 + 20 + 10].to_vec();
        let parsed = parse_packet(cut, "eth0".into());
        assert!(parsed.ipv4.is_some());
        assert!(parsed.tcp.is_none());
        assert!(parsed.payload.is_empty());
    }

    #[test]
    fn raw_ipv4_without_ethernet_is_parsed() {
        let raw = ipv4_segment(
            [1, 2, 3, 4],
            [5, 6, 7, 8],
            6,
            &tcp_segment(1111, 2222, b"raw"),
        );
        let parsed = parse_packet_with_linktype(raw, "any".into(), LINKTYPE_RAW);
        assert!(parsed.ethernet.is_none());
        let ip = parsed.ipv4.expect("ipv4");
        assert_eq!(ip.src_ip, "1.2.3.4");
        assert_eq!(parsed.tcp.expect("tcp").src_port, 1111);
        assert_eq!(parsed.payload, b"raw");
    }

    #[test]
    fn raw_ipv6_without_ethernet_is_parsed() {
        let mut ip6 = Vec::new();
        ip6.push(0x60);
        ip6.extend_from_slice(&[0x00, 0x00, 0x00]);
        ip6.extend_from_slice(&(11u16).to_be_bytes());
        ip6.push(17);
        ip6.push(64);
        ip6.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1]);
        ip6.extend_from_slice(&[0x20, 0x01, 0x0d, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2]);
        ip6.extend_from_slice(&udp_segment(1, 2, b"abc"));
        let parsed = parse_packet_with_linktype(ip6, "any".into(), LINKTYPE_RAW);
        assert!(parsed.ethernet.is_none());
        let ip = parsed.ipv6.expect("ipv6");
        assert_eq!(ip.src_ip, "2001:db8::1");
        assert_eq!(ip.next_header, "UDP");
        assert_eq!(parsed.udp.expect("udp").dst_port, 2);
        assert_eq!(parsed.payload, b"abc");
    }

    #[test]
    fn non_ip_frame_falls_back_to_raw_payload() {
        let frame: Vec<u8> = b"not an ip packet".to_vec();
        assert!(frame.len() >= 14);
        let parsed = parse_packet(frame.clone(), "eth0".into());
        assert!(parsed.ethernet.is_some());
        assert!(parsed.ipv4.is_none());
        assert!(parsed.ipv6.is_none());
        assert_eq!(parsed.payload, frame);
    }

    #[test]
    fn short_frame_is_not_misparsed() {
        let parsed = parse_packet(b"short".to_vec(), "eth0".into());
        assert!(parsed.ethernet.is_none());
        assert!(parsed.ipv4.is_none());
        assert_eq!(parsed.payload, b"short");
    }

    #[test]
    fn hex_round_trip() {
        let bytes: Vec<u8> = (0u8..=255).collect();
        let hex = bytes_to_hex(&bytes);
        let decoded = hex_to_bytes(&hex).expect("decode");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn hex_to_bytes_handles_whitespace_and_errors() {
        assert_eq!(
            hex_to_bytes("de ad be ef").expect("ok"),
            vec![0xde, 0xad, 0xbe, 0xef]
        );
        assert_eq!(hex_to_bytes("").expect("ok"), Vec::<u8>::new());
        assert!(hex_to_bytes("abc").is_err());
        assert!(hex_to_bytes("zz").is_err());
    }

    #[test]
    fn bytes_to_ascii_escapes_non_printable() {
        assert_eq!(bytes_to_ascii(b"A\x00B\x7fC"), "A.B.C");
    }
}
