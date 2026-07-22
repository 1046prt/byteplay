use log::{error, info, warn};
use pnet::datalink::{self, Channel, Config};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::parser::parse_packet;

const MAX_STORED_PACKETS: usize = 50_000;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CaptureConfig {
    pub interface_name: String,
    pub bpf_filter: String,
    pub max_packets: Option<u32>,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct CapturedPacket {
    pub id: String,
    pub timestamp: String,
    pub interface: String,
    pub frame_length: usize,
    pub ethernet: Option<crate::parser::EthernetInfo>,
    pub ipv4: Option<crate::parser::IPv4Info>,
    pub ipv6: Option<crate::parser::IPv6Info>,
    pub tcp: Option<crate::parser::TcpInfo>,
    pub udp: Option<crate::parser::UdpInfo>,
    pub raw_bytes: Vec<u8>,
    pub payload: Vec<u8>,
    pub payload_hex: String,
    pub payload_ascii: String,
    pub capture_index: usize,
}

pub struct CaptureEngine {
    running: Arc<AtomicBool>,
    packets: Arc<Mutex<Vec<CapturedPacket>>>,
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            packets: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn list_interfaces() -> Vec<String> {
        datalink::interfaces()
            .iter()
            .map(|iface| {
                let addrs: Vec<String> = iface
                    .ips
                    .iter()
                    .map(|ip_network| ip_network.ip().to_string())
                    .collect();
                if addrs.is_empty() {
                    iface.name.clone()
                } else {
                    format!("{} [{}]", iface.name, addrs.join(", "))
                }
            })
            .collect()
    }

    pub fn list_interface_names() -> Vec<String> {
        datalink::interfaces()
            .iter()
            .map(|iface| iface.name.clone())
            .collect()
    }

    pub fn start_capture(
        &self,
        config: CaptureConfig,
        tx: std::sync::mpsc::Sender<CapturedPacket>,
    ) -> Result<(), String> {
        if self.running.load(Ordering::SeqCst) {
            return Err("Capture already running".to_string());
        }

        let interfaces = datalink::interfaces();
        let interface = interfaces
            .iter()
            .find(|i| i.name == config.interface_name)
            .ok_or_else(|| format!("Interface '{}' not found", config.interface_name))?;

        let mut link_config = Config::default();
        link_config.read_timeout = Some(std::time::Duration::from_millis(100));

        let mut rx = match datalink::channel(&interface, link_config)
            .map_err(|e| format!("Failed to open channel on {}: {}", config.interface_name, e))?
        {
            Channel::Ethernet(_, rx) => rx,
            _ => return Err("Unsupported channel type".to_string()),
        };

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        let filter = config.bpf_filter.trim().to_string();
        if !filter.is_empty() {
            warn!(
                "BPF filter '{}' cannot be applied at kernel level via pnet; using application-level filtering",
                filter
            );
        }

        thread::spawn(move || {
            info!("Capture started on interface: {}", config.interface_name);
            if !filter.is_empty() {
                info!("Application-level filter active: {}", filter);
            }
            let mut count = 0usize;
            let mut matched = 0usize;

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match rx.next() {
                    Ok(packet) => {
                        count += 1;
                        let parsed = parse_packet(packet.to_vec(), config.interface_name.clone());

                        if !filter.is_empty() && !matches_filter(&parsed, &filter) {
                            continue;
                        }

                        matched += 1;
                        let captured = CapturedPacket {
                            id: parsed.id,
                            timestamp: parsed.timestamp,
                            interface: parsed.interface,
                            frame_length: parsed.frame_length,
                            ethernet: parsed.ethernet,
                            ipv4: parsed.ipv4,
                            ipv6: parsed.ipv6,
                            tcp: parsed.tcp,
                            udp: parsed.udp,
                            raw_bytes: parsed.raw_bytes,
                            payload: parsed.payload,
                            payload_hex: parsed.payload_hex,
                            payload_ascii: parsed.payload_ascii,
                            capture_index: matched,
                        };

                        if tx.send(captured).is_err() {
                            error!("Failed to send captured packet - receiver dropped");
                            break;
                        }

                        if let Some(max) = config.max_packets {
                            if matched >= max as usize {
                                info!("Reached max packet limit: {}", max);
                                break;
                            }
                        }
                    }
                    Err(ref e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock
                            || e.kind() == std::io::ErrorKind::TimedOut
                        {
                            continue;
                        }
                        warn!("Capture error: {}", e);
                    }
                }
            }

            running.store(false, Ordering::SeqCst);
            info!(
                "Capture stopped. Total seen: {}, matched filter: {}",
                count, matched
            );
        });

        Ok(())
    }

    pub fn stop_capture(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    pub fn store_packet(&self, packet: CapturedPacket) {
        let mut pkts = match self.packets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        if pkts.len() >= MAX_STORED_PACKETS {
            pkts.drain(..MAX_STORED_PACKETS / 10);
        }
        pkts.push(packet);
    }

    pub fn get_packets(&self) -> Vec<CapturedPacket> {
        match self.packets.lock() {
            Ok(pkts) => pkts.clone(),
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner().clone()
            }
        }
    }

    pub fn get_packet_by_id(&self, id: &str) -> Option<CapturedPacket> {
        let pkts = match self.packets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };
        pkts.iter().find(|p| p.id == id).cloned()
    }

    pub fn clear_packets(&self) {
        match self.packets.lock() {
            Ok(mut pkts) => pkts.clear(),
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner().clear();
            }
        }
    }
}

fn matches_filter(packet: &crate::parser::ParsedPacket, filter: &str) -> bool {
    let lower = filter.to_lowercase();
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    for token in &tokens {
        let t = *token;
        match t {
            "tcp" => {
                if packet.tcp.is_none() {
                    return false;
                }
            }
            "udp" => {
                if packet.udp.is_none() {
                    return false;
                }
            }
            "icmp" | "icmpv6" => {
                if packet.tcp.is_none() && packet.udp.is_none() {
                    // No transport layer — likely ICMP
                } else {
                    return false;
                }
            }
            _ => {
                // Try port number filter
                if let Ok(port) = t.parse::<u16>() {
                    let src_match = packet
                        .tcp
                        .as_ref()
                        .map(|t| t.src_port == port)
                        .or_else(|| packet.udp.as_ref().map(|u| u.src_port == port))
                        .unwrap_or(false);
                    let dst_match = packet
                        .tcp
                        .as_ref()
                        .map(|t| t.dst_port == port)
                        .or_else(|| packet.udp.as_ref().map(|u| u.dst_port == port))
                        .unwrap_or(false);
                    if !src_match && !dst_match {
                        return false;
                    }
                }
                // Try IP address filter
                else if let Some(ip_str) = t.strip_prefix("host ") {
                    let src_match = packet
                        .ipv4
                        .as_ref()
                        .map(|ip| ip.src_ip == ip_str)
                        .unwrap_or(false)
                        || packet
                            .ipv6
                            .as_ref()
                            .map(|ip| ip.src_ip == ip_str)
                            .unwrap_or(false);
                    let dst_match = packet
                        .ipv4
                        .as_ref()
                        .map(|ip| ip.dst_ip == ip_str)
                        .unwrap_or(false)
                        || packet
                            .ipv6
                            .as_ref()
                            .map(|ip| ip.dst_ip == ip_str)
                            .unwrap_or(false);
                    if !src_match && !dst_match {
                        return false;
                    }
                }
                // Bare IP address
                else if t.contains('.') || t.contains(':') {
                    let src_match = packet
                        .ipv4
                        .as_ref()
                        .map(|ip| ip.src_ip == t)
                        .unwrap_or(false)
                        || packet
                            .ipv6
                            .as_ref()
                            .map(|ip| ip.src_ip == t)
                            .unwrap_or(false);
                    let dst_match = packet
                        .ipv4
                        .as_ref()
                        .map(|ip| ip.dst_ip == t)
                        .unwrap_or(false)
                        || packet
                            .ipv6
                            .as_ref()
                            .map(|ip| ip.dst_ip == t)
                            .unwrap_or(false);
                    if !src_match && !dst_match {
                        return false;
                    }
                }
            }
        }
    }
    true
}
