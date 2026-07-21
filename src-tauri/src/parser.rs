use pnet::packet::{
    ethernet::{EtherTypes, EthernetPacket},
    ip::IpNextHeaderProtocols,
    ipv4::Ipv4Packet,
    ipv6::Ipv6Packet,
    tcp::TcpPacket,
    udp::UdpPacket,
    Packet,
};
use pnet::packet::ip::IpNextHeaderProtocol;
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

pub fn parse_packet(raw_bytes: Vec<u8>, interface: String) -> ParsedPacket {
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

    if let Some(eth) = parse_ethernet(&raw_bytes) {
        result.ethernet = Some(EthernetInfo {
            src_mac: format_mac(eth.get_source()),
            dst_mac: format_mac(eth.get_destination()),
            ether_type: format!("{:?}", eth.get_ethertype()),
        });

        match eth.get_ethertype() {
            EtherTypes::Ipv4 => {
                if let Some(ipv4) = Ipv4Packet::new(eth.payload()) {
                    let protocol = format_ip_protocol(ipv4.get_next_level_protocol());
                    let payload = ipv4.payload().to_vec();
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
                            if let Some(tcp) = TcpPacket::new(&payload) {
                                result.tcp = Some(parse_tcp(&tcp));
                                result.payload = tcp.payload().to_vec();
                            }
                        }
                        IpNextHeaderProtocols::Udp => {
                            if let Some(udp) = UdpPacket::new(&payload) {
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
                            result.payload = payload;
                        }
                    }
                }
            }
            EtherTypes::Ipv6 => {
                if let Some(ipv6) = Ipv6Packet::new(eth.payload()) {
                    let protocol = format_ip_protocol(ipv6.get_next_header());
                    let payload = ipv6.payload().to_vec();
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
                            if let Some(tcp) = TcpPacket::new(&payload) {
                                result.tcp = Some(parse_tcp(&tcp));
                                result.payload = tcp.payload().to_vec();
                            }
                        }
                        IpNextHeaderProtocols::Udp => {
                            if let Some(udp) = UdpPacket::new(&payload) {
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
                            result.payload = payload;
                        }
                    }
                }
            }
            _ => {
                result.payload = raw_bytes;
            }
        }
    } else {
        result.payload = raw_bytes;
    }

    result.payload_hex = bytes_to_hex(&result.payload);
    result.payload_ascii = bytes_to_ascii(&result.payload);

    result
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
            if b >= 0x20 && b <= 0x7e {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

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
