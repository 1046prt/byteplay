use log::info;
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::ToSocketAddrs;
use std::time::Duration;

const RESPONSE_BUFFER_SIZE: usize = 65535;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub target_host: String,
    pub target_port: u16,
    pub protocol: String,
    pub timeout_ms: u64,
    pub allow_external: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayResult {
    pub success: bool,
    pub bytes_sent: usize,
    pub response: Option<Vec<u8>>,
    pub response_hex: Option<String>,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub target: String,
    pub timestamp: String,
}

fn is_local_address(host: &str) -> bool {
    if host == "localhost" || host == "127.0.0.1" || host == "::1" || host == "0.0.0.0" {
        return true;
    }

    if host.starts_with("10.") || host.starts_with("172.") || host.starts_with("192.168.") {
        return true;
    }

    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(v4) => {
                return v4.is_loopback() || v4.is_private() || v4.is_link_local();
            }
            std::net::IpAddr::V6(v6) => {
                return v6.is_loopback() || v6.is_unspecified();
            }
        }
    }

    false
}

pub fn replay_packet(data: &[u8], config: ReplayConfig) -> ReplayResult {
    let start = std::time::Instant::now();
    let timestamp = chrono::Utc::now().to_rfc3339();
    let target = format!("{}:{}", config.target_host, config.target_port);

    if !config.allow_external && !is_local_address(&config.target_host) {
        return ReplayResult {
            success: false,
            bytes_sent: 0,
            response: None,
            response_hex: None,
            error: Some(format!(
                "Blocked: target {} is not a local/private address. Use allow_external=true to override.",
                config.target_host
            )),
            duration_ms: start.elapsed().as_millis() as u64,
            target,
            timestamp,
        };
    }

    let result = match config.protocol.to_uppercase().as_str() {
        "TCP" => send_tcp(data, &config),
        "UDP" => send_udp(data, &config),
        _ => Err(format!("Unsupported protocol: {}", config.protocol)),
    };

    match result {
        Ok(resp) => {
            let duration = start.elapsed().as_millis() as u64;
            info!("Replay to {} completed in {}ms", target, duration);
            ReplayResult {
                success: true,
                bytes_sent: data.len(),
                response: resp.as_ref().map(|r| r.clone()),
                response_hex: resp.as_ref().map(|r| {
                    r.iter()
                        .map(|b| format!("{:02x}", b))
                        .collect::<Vec<_>>()
                        .join(" ")
                }),
                error: None,
                duration_ms: duration,
                target,
                timestamp,
            }
        }
        Err(e) => {
            let duration = start.elapsed().as_millis() as u64;
            ReplayResult {
                success: false,
                bytes_sent: 0,
                response: None,
                response_hex: None,
                error: Some(e),
                duration_ms: duration,
                target,
                timestamp,
            }
        }
    }
}

fn send_tcp(data: &[u8], config: &ReplayConfig) -> Result<Option<Vec<u8>>, String> {
    let addr = format!("{}:{}", config.target_host, config.target_port)
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address: {}", e))?
        .next()
        .ok_or_else(|| "No addresses resolved".to_string())?;

    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };

    let socket = Socket::new(domain, Type::STREAM, Some(Protocol::TCP))
        .map_err(|e| format!("Failed to create TCP socket: {}", e))?;

    socket
        .set_read_timeout(Some(Duration::from_millis(config.timeout_ms)))
        .map_err(|e| format!("Failed to set read timeout: {}", e))?;

    socket
        .set_write_timeout(Some(Duration::from_millis(config.timeout_ms)))
        .map_err(|e| format!("Failed to set write timeout: {}", e))?;

    socket
        .connect(&addr.into())
        .map_err(|e| format!("TCP connect failed: {}", e))?;

    let mut std_socket: std::net::TcpStream = socket.into();
    std_socket
        .write_all(data)
        .map_err(|e| format!("TCP write failed: {}", e))?;

    let mut buf = vec![0u8; RESPONSE_BUFFER_SIZE];
    let mut response = Vec::new();

    match std_socket.read(&mut buf) {
        Ok(n) => {
            response.extend_from_slice(&buf[..n]);
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock
            || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            info!("TCP read timed out - no response received");
        }
        Err(e) => {
            info!("TCP read error (may be normal for one-way send): {}", e);
        }
    }

    Ok(if response.is_empty() {
        None
    } else {
        Some(response)
    })
}

use std::io::Read;

fn send_udp(data: &[u8], config: &ReplayConfig) -> Result<Option<Vec<u8>>, String> {
    let addr = format!("{}:{}", config.target_host, config.target_port)
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve address: {}", e))?
        .next()
        .ok_or_else(|| "No addresses resolved".to_string())?;

    let domain = if addr.is_ipv4() {
        Domain::IPV4
    } else {
        Domain::IPV6
    };

    let socket = Socket::new(domain, Type::DGRAM, Some(Protocol::UDP))
        .map_err(|e| format!("Failed to create UDP socket: {}", e))?;

    socket
        .set_read_timeout(Some(Duration::from_millis(config.timeout_ms)))
        .map_err(|e| format!("Failed to set read timeout: {}", e))?;

    socket
        .send_to(data, &addr.into())
        .map_err(|e| format!("UDP send failed: {}", e))?;

    let std_socket: std::net::UdpSocket = socket.into();
    let mut buf = vec![0u8; RESPONSE_BUFFER_SIZE];

    match std_socket.recv_from(&mut buf) {
        Ok((n, _)) => {
            Ok(Some(buf[..n].to_vec()))
        }
        Err(ref e)
            if e.kind() == std::io::ErrorKind::WouldBlock
                || e.kind() == std::io::ErrorKind::TimedOut =>
        {
            info!("UDP read timed out - no response received");
            Ok(None)
        }
        Err(e) => Err(format!("UDP recv failed: {}", e)),
    }
}

use std::io::Write;

pub fn execute_sequence(
    steps: Vec<SequenceStep>,
    allow_external: bool,
) -> Vec<ReplayResult> {
    let mut results = Vec::new();

    for step in steps {
        if step.delay_ms > 0 && !results.is_empty() {
            std::thread::sleep(Duration::from_millis(step.delay_ms));
        }

        let config = ReplayConfig {
            target_host: step.target_host.clone(),
            target_port: step.target_port,
            protocol: step.protocol.clone(),
            timeout_ms: step.timeout_ms.unwrap_or(5000),
            allow_external,
        };

        let result = replay_packet(&step.data, config);
        results.push(result);
    }

    results
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SequenceStep {
    pub name: String,
    pub target_host: String,
    pub target_port: u16,
    pub protocol: String,
    pub data: Vec<u8>,
    pub delay_ms: u64,
    pub timeout_ms: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzzConfig {
    pub target_host: String,
    pub target_port: u16,
    pub protocol: String,
    pub base_payload: Vec<u8>,
    pub iterations: u32,
    pub mutation_rate: f64,
    pub timeout_ms: u64,
    pub allow_external: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FuzzResult {
    pub iteration: u32,
    pub mutated_payload: Vec<u8>,
    pub replay_result: ReplayResult,
    pub mutations_applied: Vec<Mutation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Mutation {
    pub offset: usize,
    pub original: u8,
    pub mutated: u8,
}

pub fn run_fuzz(config: FuzzConfig, progress_tx: Option<std::sync::mpsc::Sender<FuzzResult>>) -> Vec<FuzzResult> {
    let mut results = Vec::new();

    for i in 0..config.iterations {
        let (mutated, mutations) = mutate_payload(&config.base_payload, config.mutation_rate);

        let replay_config = ReplayConfig {
            target_host: config.target_host.clone(),
            target_port: config.target_port,
            protocol: config.protocol.clone(),
            timeout_ms: config.timeout_ms,
            allow_external: config.allow_external,
        };

        let result = replay_packet(&mutated, replay_config);
        let fuzz_result = FuzzResult {
            iteration: i + 1,
            mutated_payload: mutated,
            replay_result: result,
            mutations_applied: mutations,
        };

        if let Some(ref tx) = progress_tx {
            if tx.send(fuzz_result.clone()).is_err() {
                break;
            }
        }

        results.push(fuzz_result);
    }

    results
}

fn mutate_payload(base: &[u8], rate: f64) -> (Vec<u8>, Vec<Mutation>) {
    let mut mutated = base.to_vec();
    let mut mutations = Vec::new();

    for (i, byte) in mutated.iter_mut().enumerate() {
        if rand::random::<f64>() < rate {
            let original = *byte;
            let mutation_type = rand::random::<u8>() % 4;
            match mutation_type {
                0 => *byte = rand::random::<u8>(),
                1 => *byte = 0x00,
                2 => *byte = 0xFF,
                3 => *byte = original.wrapping_add(1),
                _ => {}
            }
            if *byte != original {
                mutations.push(Mutation {
                    offset: i,
                    original,
                    mutated: *byte,
                });
            }
        }
    }

    (mutated, mutations)
}
