use log::{error, info, warn};
use pnet::datalink::{self, Config};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use crate::parser::parse_packet;

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

const MAX_STORED_PACKETS: usize = 10000;

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
                    .addresses
                    .iter()
                    .filter_map(|addr| match addr {
                        datalink::NetworkInterfaceIndex::Ipv4(ip) => Some(ip.to_string()),
                        datalink::NetworkInterfaceIndex::Ipv6(ip) => Some(ip.to_string()),
                        _ => None,
                    })
                    .collect();
                if addrs.is_empty() {
                    iface.name.clone()
                } else {
                    format!("{} [{}]", iface.name, addrs.join(", "))
                }
            })
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
        link_config.read_timeout = std::time::Duration::from_millis(100);

        let (_, mut rx) = datalink::channel(&interface, link_config)
            .map_err(|e| format!("Failed to open channel on {}: {}", config.interface_name, e))?;

        self.running.store(true, Ordering::SeqCst);
        let running = self.running.clone();

        thread::spawn(move || {
            info!("Capture started on interface: {}", config.interface_name);
            let mut count = 0usize;

            loop {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                match rx.next() {
                    Ok(packet) => {
                        count += 1;
                        let parsed = parse_packet(packet.to_vec(), config.interface_name.clone());
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
                            capture_index: count,
                        };

                        if tx.send(captured).is_err() {
                            error!("Failed to send captured packet - receiver dropped");
                            break;
                        }

                        if let Some(max) = config.max_packets {
                            if count >= max as usize {
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
            info!("Capture stopped. Total packets: {}", count);
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
        if let Ok(mut packets) = self.packets.lock() {
            if packets.len() >= MAX_STORED_PACKETS {
                packets.remove(0);
            }
            packets.push(packet);
        }
    }

    pub fn get_packets(&self) -> Vec<CapturedPacket> {
        self.packets
            .lock()
            .map(|packets| packets.clone())
            .unwrap_or_default()
    }

    pub fn get_packet_by_id(&self, id: &str) -> Option<CapturedPacket> {
        self.packets
            .lock()
            .ok()
            .and_then(|packets| {
                packets.iter().find(|p| p.id == id).cloned()
            })
    }

    pub fn clear_packets(&self) {
        if let Ok(mut packets) = self.packets.lock() {
            packets.clear();
        }
    }
}
