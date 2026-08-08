use log::{error, info, warn};
use pnet::datalink::{self, Channel, Config};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    pub seq: u64,
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

#[derive(Clone, Debug, serde::Serialize)]
pub struct PollBatch {
    pub packets: Vec<CapturedPacket>,
    pub latest_seq: u64,
    pub dropped: u64,
}

pub struct StatsMaps {
    pub protocols: HashMap<String, (usize, usize)>,
    pub sources: HashMap<String, usize>,
    pub destinations: HashMap<String, usize>,
    pub timeline: BTreeMap<String, (usize, usize)>,
}

pub struct CaptureEngine {
    running: Arc<AtomicBool>,
    packets: Arc<Mutex<Vec<CapturedPacket>>>,
    next_seq: AtomicUsize,
    total_packets: AtomicUsize,
    total_bytes: AtomicUsize,
    protocols: Arc<Mutex<HashMap<String, (usize, usize)>>>,
    sources: Arc<Mutex<HashMap<String, usize>>>,
    destinations: Arc<Mutex<HashMap<String, usize>>>,
    timeline: Arc<Mutex<BTreeMap<String, (usize, usize)>>>,
}

impl CaptureEngine {
    pub fn new() -> Self {
        Self {
            running: Arc::new(AtomicBool::new(false)),
            packets: Arc::new(Mutex::new(Vec::new())),
            next_seq: AtomicUsize::new(0),
            total_packets: AtomicUsize::new(0),
            total_bytes: AtomicUsize::new(0),
            protocols: Arc::new(Mutex::new(HashMap::new())),
            sources: Arc::new(Mutex::new(HashMap::new())),
            destinations: Arc::new(Mutex::new(HashMap::new())),
            timeline: Arc::new(Mutex::new(BTreeMap::new())),
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

        let link_config = Config {
            read_timeout: Some(std::time::Duration::from_millis(100)),
            ..Config::default()
        };

        let mut rx = match datalink::channel(interface, link_config)
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
                            seq: 0,
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

    pub fn store_packet(&self, mut packet: CapturedPacket) {
        packet.seq = self.next_seq.fetch_add(1, Ordering::SeqCst) as u64;
        self.total_packets.fetch_add(1, Ordering::SeqCst);
        self.total_bytes
            .fetch_add(packet.frame_length, Ordering::SeqCst);
        self.record_stats(&packet);

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

    pub fn totals(&self) -> (usize, usize) {
        (
            self.total_packets.load(Ordering::SeqCst),
            self.total_bytes.load(Ordering::SeqCst),
        )
    }

    pub fn poll_since(&self, since: u64) -> PollBatch {
        let pkts = match self.packets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner()
            }
        };

        let latest_seq = pkts.last().map(|p| p.seq).unwrap_or(since);
        let start = if since == 0 {
            0
        } else {
            pkts.partition_point(|p| p.seq <= since)
        };

        let dropped = pkts
            .first()
            .filter(|p| p.seq > since)
            .map(|p| p.seq.saturating_sub(since + 1))
            .unwrap_or(0);

        PollBatch {
            packets: pkts[start..].to_vec(),
            latest_seq: latest_seq.max(since),
            dropped,
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
        self.next_seq.store(0, Ordering::SeqCst);
        self.total_packets.store(0, Ordering::SeqCst);
        self.total_bytes.store(0, Ordering::SeqCst);
        match self.packets.lock() {
            Ok(mut pkts) => pkts.clear(),
            Err(poisoned) => {
                warn!("Mutex poisoned, recovering");
                poisoned.into_inner().clear();
            }
        }
        match self.protocols.lock() {
            Ok(mut m) => m.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        match self.sources.lock() {
            Ok(mut m) => m.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        match self.destinations.lock() {
            Ok(mut m) => m.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
        match self.timeline.lock() {
            Ok(mut m) => m.clear(),
            Err(poisoned) => poisoned.into_inner().clear(),
        }
    }

    pub fn stats_maps(&self) -> StatsMaps {
        StatsMaps {
            protocols: self.protocols.lock().map(|g| g.clone()).unwrap_or_default(),
            sources: self.sources.lock().map(|g| g.clone()).unwrap_or_default(),
            destinations: self
                .destinations
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default(),
            timeline: self.timeline.lock().map(|g| g.clone()).unwrap_or_default(),
        }
    }

    fn record_stats(&self, packet: &CapturedPacket) {
        let protocol = protocol_label(packet);
        if let Ok(mut map) = self.protocols.lock() {
            let entry = map.entry(protocol).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += packet.frame_length;
        }

        let src_ep = endpoint_label(packet, true);
        if !src_ep.is_empty() {
            if let Ok(mut map) = self.sources.lock() {
                *map.entry(src_ep).or_insert(0) += 1;
            }
        }

        let dst_ep = endpoint_label(packet, false);
        if !dst_ep.is_empty() {
            if let Ok(mut map) = self.destinations.lock() {
                *map.entry(dst_ep).or_insert(0) += 1;
            }
        }

        let minute = minute_bucket(&packet.timestamp);
        if let Ok(mut map) = self.timeline.lock() {
            let entry = map.entry(minute).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += packet.frame_length;
        }
    }
}

fn protocol_label(packet: &CapturedPacket) -> String {
    if packet.tcp.is_some() {
        "TCP".to_string()
    } else if packet.udp.is_some() {
        "UDP".to_string()
    } else if let Some(ref ip4) = packet.ipv4 {
        ip4.protocol.clone()
    } else if let Some(ref ip6) = packet.ipv6 {
        ip6.next_header.clone()
    } else {
        "Other".to_string()
    }
}

fn endpoint_label(packet: &CapturedPacket, source: bool) -> String {
    let ip = if source {
        packet
            .ipv4
            .as_ref()
            .map(|ip| ip.src_ip.clone())
            .or_else(|| packet.ipv6.as_ref().map(|ip| ip.src_ip.clone()))
            .unwrap_or_default()
    } else {
        packet
            .ipv4
            .as_ref()
            .map(|ip| ip.dst_ip.clone())
            .or_else(|| packet.ipv6.as_ref().map(|ip| ip.dst_ip.clone()))
            .unwrap_or_default()
    };

    if ip.is_empty() {
        return String::new();
    }

    let port = if source {
        packet
            .tcp
            .as_ref()
            .map(|t| t.src_port)
            .or_else(|| packet.udp.as_ref().map(|u| u.src_port))
    } else {
        packet
            .tcp
            .as_ref()
            .map(|t| t.dst_port)
            .or_else(|| packet.udp.as_ref().map(|u| u.dst_port))
    };

    match port {
        Some(port) => format!("{}:{}", ip, port),
        None => ip,
    }
}

fn minute_bucket(timestamp: &str) -> String {
    chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_else(|_| "??".to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn packet(seq: u64) -> CapturedPacket {
        CapturedPacket {
            id: format!("p{}", seq),
            seq,
            timestamp: String::new(),
            interface: "test".into(),
            frame_length: 0,
            ethernet: None,
            ipv4: None,
            ipv6: None,
            tcp: None,
            udp: None,
            raw_bytes: Vec::new(),
            payload: Vec::new(),
            payload_hex: String::new(),
            payload_ascii: String::new(),
            capture_index: 0,
        }
    }

    #[test]
    fn store_assigns_monotonic_seqs() {
        let engine = CaptureEngine::new();
        engine.store_packet(packet(0));
        engine.store_packet(packet(0));
        engine.store_packet(packet(0));
        let batch = engine.poll_since(0);
        assert_eq!(batch.packets.len(), 3);
        assert_eq!(batch.packets[0].seq, 0);
        assert_eq!(batch.packets[2].seq, 2);
        assert_eq!(batch.latest_seq, 2);
        assert_eq!(batch.dropped, 0);
    }

    #[test]
    fn poll_since_returns_only_new_packets() {
        let engine = CaptureEngine::new();
        for _ in 0..10 {
            engine.store_packet(packet(0));
        }
        let first = engine.poll_since(0);
        assert_eq!(first.packets.len(), 10);
        assert_eq!(first.latest_seq, 9);

        let second = engine.poll_since(9);
        assert!(second.packets.is_empty());
        assert_eq!(second.latest_seq, 9);
    }

    #[test]
    fn poll_since_detects_evicted_packets() {
        let engine = CaptureEngine::new();
        for _ in 0..10 {
            engine.store_packet(packet(0));
        }
        engine.poll_since(0);
        // Evict the first 6 (seqs 0..=5), simulate client cursor at 2
        engine.packets.lock().unwrap().drain(..6);
        let batch = engine.poll_since(2);
        assert_eq!(batch.dropped, 3);
        assert_eq!(batch.packets.first().unwrap().seq, 6);
        assert_eq!(batch.latest_seq, 9);
    }

    #[test]
    fn poll_since_jumped_cursor_returns_nothing() {
        let engine = CaptureEngine::new();
        for _ in 0..5 {
            engine.store_packet(packet(0));
        }
        let batch = engine.poll_since(100);
        assert!(batch.packets.is_empty());
        assert_eq!(batch.latest_seq, 100);
        assert_eq!(batch.dropped, 0);
    }

    #[test]
    fn clear_resets_sequence_and_totals() {
        let engine = CaptureEngine::new();
        for _ in 0..5 {
            engine.store_packet(packet(0));
        }
        assert_eq!(engine.totals(), (5, 0));
        engine.clear_packets();
        assert_eq!(engine.totals(), (0, 0));
        engine.store_packet(packet(0));
        assert_eq!(engine.poll_since(0).packets[0].seq, 0);
    }

    #[test]
    fn ring_buffer_evicts_oldest_ten_percent() {
        let engine = CaptureEngine::new();
        for _ in 0..(MAX_STORED_PACKETS + 100) {
            let mut p = packet(0);
            p.frame_length = 1;
            engine.store_packet(p);
        }
        let expected = MAX_STORED_PACKETS - MAX_STORED_PACKETS / 10 + 100;
        let batch = engine.poll_since(0);
        assert_eq!(batch.packets.len(), expected);
        assert_eq!(
            batch.packets.first().unwrap().seq,
            (MAX_STORED_PACKETS / 10) as u64
        );
        assert_eq!(engine.totals().0, MAX_STORED_PACKETS + 100);
        assert_eq!(engine.totals().1, MAX_STORED_PACKETS + 100);
    }

    fn tcp_packet(src: &str, dst: &str, src_port: u16, dst_port: u16) -> CapturedPacket {
        let mut p = packet(0);
        p.timestamp = "2026-01-01T10:05:00Z".to_string();
        p.frame_length = 100;
        p.ipv4 = Some(crate::parser::IPv4Info {
            src_ip: src.to_string(),
            dst_ip: dst.to_string(),
            version: 4,
            ihl: 5,
            dscp: 0,
            ecn: 0,
            total_length: 0,
            identification: 0,
            flags: 0,
            fragment_offset: 0,
            ttl: 64,
            protocol: "TCP".to_string(),
            checksum: 0,
        });
        p.tcp = Some(crate::parser::TcpInfo {
            src_port,
            dst_port,
            sequence: 0,
            ack_number: 0,
            data_offset: 0,
            flags: crate::parser::TcpFlags {
                syn: false,
                ack: false,
                fin: false,
                rst: false,
                psh: false,
                urg: false,
            },
            window: 0,
            checksum: 0,
            urgent_pointer: 0,
        });
        p
    }

    #[test]
    fn stats_counters_accumulate_per_packet() {
        let engine = CaptureEngine::new();
        engine.store_packet(tcp_packet("10.0.0.1", "10.0.0.2", 1234, 80));
        engine.store_packet(tcp_packet("10.0.0.1", "10.0.0.2", 1234, 80));
        engine.store_packet(tcp_packet("10.0.0.3", "10.0.0.2", 4321, 80));
        engine.store_packet(packet(0));

        let maps = engine.stats_maps();
        assert_eq!(maps.protocols.get("TCP").map(|(c, _)| *c), Some(3));
        assert_eq!(maps.protocols.get("TCP").map(|(_, b)| *b), Some(300));
        assert_eq!(maps.protocols.get("Other").map(|(c, _)| *c), Some(1));
        assert_eq!(maps.sources.get("10.0.0.1:1234"), Some(&2));
        assert_eq!(maps.sources.get("10.0.0.3:4321"), Some(&1));
        assert_eq!(maps.destinations.get("10.0.0.2:80"), Some(&3));
        assert_eq!(
            maps.timeline.get("10:05").map(|(c, b)| (*c, *b)),
            Some((3, 300))
        );
    }

    #[test]
    fn stats_ignore_unparsed_packets() {
        let engine = CaptureEngine::new();
        let mut p = packet(0);
        p.timestamp = "not-a-timestamp".to_string();
        p.frame_length = 50;
        engine.store_packet(p);

        let maps = engine.stats_maps();
        assert_eq!(maps.protocols.get("Other").map(|(c, _)| *c), Some(1));
        assert_eq!(maps.timeline.get("??").map(|(c, _)| *c), Some(1));
        assert!(maps.sources.is_empty());
        assert!(maps.destinations.is_empty());
    }

    #[test]
    fn stats_reset_on_clear() {
        let engine = CaptureEngine::new();
        engine.store_packet(tcp_packet("10.0.0.1", "10.0.0.2", 1234, 80));
        engine.clear_packets();
        let maps = engine.stats_maps();
        assert!(maps.protocols.is_empty());
        assert!(maps.sources.is_empty());
        assert!(maps.destinations.is_empty());
        assert!(maps.timeline.is_empty());
        assert_eq!(engine.totals(), (0, 0));
    }

    #[test]
    fn stats_continue_across_ring_buffer_eviction() {
        let engine = CaptureEngine::new();
        let p = tcp_packet("10.0.0.1", "10.0.0.2", 1234, 80);
        for _ in 0..(MAX_STORED_PACKETS * 2) {
            engine.store_packet(p.clone());
        }
        let maps = engine.stats_maps();
        assert_eq!(
            maps.protocols.get("TCP").map(|(c, _)| *c),
            Some(MAX_STORED_PACKETS * 2)
        );
        assert_eq!(engine.totals().0, MAX_STORED_PACKETS * 2);
    }
}
