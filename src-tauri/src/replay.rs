use log::info;
use rand::Rng;
use serde::{Deserialize, Serialize};
use socket2::{Domain, Protocol, Socket, Type};
use std::net::ToSocketAddrs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const RESPONSE_BUFFER_SIZE: usize = 65535;
const MAX_FUZZ_ITERATIONS: u32 = 10_000;
const MAX_FUZZ_TIMEOUT_MS: u64 = 30_000;

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
                response: resp.clone(),
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
        Err(ref e)
            if e.kind() == std::io::ErrorKind::WouldBlock
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
        Ok((n, _)) => Ok(Some(buf[..n].to_vec())),
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

pub fn execute_sequence(steps: Vec<SequenceStep>, allow_external: bool) -> Vec<ReplayResult> {
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

pub fn run_fuzz<F>(
    config: FuzzConfig,
    cancel: Option<Arc<AtomicBool>>,
    on_progress: Option<F>,
) -> Vec<FuzzResult>
where
    F: Fn(u32, u32),
{
    let mut config = config;
    clamp_fuzz_config(&mut config);

    let total = config.iterations;
    let batch = if total == 0 {
        0
    } else {
        (total / 100).clamp(1, 100)
    };

    let mut results = Vec::new();
    let mut cancelled = false;

    for i in 0..total {
        if let Some(ref flag) = cancel {
            if flag.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
        }

        let (mutated, mutations) = mutate_payload(&config.base_payload, config.mutation_rate);

        let replay_config = ReplayConfig {
            target_host: config.target_host.clone(),
            target_port: config.target_port,
            protocol: config.protocol.clone(),
            timeout_ms: config.timeout_ms,
            allow_external: config.allow_external,
        };

        let result = replay_packet(&mutated, replay_config);
        results.push(FuzzResult {
            iteration: i + 1,
            mutated_payload: mutated,
            replay_result: result,
            mutations_applied: mutations,
        });

        let done = i + 1;
        if batch > 0 && (done % batch == 0 || done == total) {
            if let Some(ref cb) = on_progress {
                cb(done, total);
            }
        }
    }

    if let Some(ref cb) = on_progress {
        if !results.is_empty() {
            let done = if cancelled {
                results.last().map(|r| r.iteration).unwrap_or(0)
            } else {
                total
            };
            cb(done, total);
        }
    }

    info!(
        "Fuzzing finished: {} of {} iterations ({})",
        results.len(),
        total,
        if cancelled { "cancelled" } else { "completed" }
    );

    results
}

fn clamp_fuzz_config(config: &mut FuzzConfig) {
    config.iterations = config.iterations.min(MAX_FUZZ_ITERATIONS);
    config.mutation_rate = config.mutation_rate.clamp(0.0, 1.0);
    config.timeout_ms = config.timeout_ms.clamp(100, MAX_FUZZ_TIMEOUT_MS);
}

fn mutate_payload(base: &[u8], rate: f64) -> (Vec<u8>, Vec<Mutation>) {
    mutate_payload_with(base, rate, &mut rand::thread_rng())
}

fn mutate_payload_with<R: Rng>(base: &[u8], rate: f64, rng: &mut R) -> (Vec<u8>, Vec<Mutation>) {
    let mut mutated = base.to_vec();
    let mut mutations = Vec::new();

    for (i, byte) in mutated.iter_mut().enumerate() {
        if rng.gen::<f64>() < rate {
            let original = *byte;
            let mutation_type = rng.gen::<u8>() % 4;
            match mutation_type {
                0 => *byte = rng.gen::<u8>(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::StdRng;
    use rand::SeedableRng;
    use std::io::{Read, Write};
    use std::net::{TcpListener, UdpSocket};

    #[test]
    fn mutate_payload_preserves_length_and_records_changes() {
        let mut rng = StdRng::seed_from_u64(42);
        let base = vec![0xAAu8; 256];
        let (mutated, mutations) = mutate_payload_with(&base, 1.0, &mut rng);
        assert_eq!(mutated.len(), base.len());
        for m in &mutations {
            assert_ne!(m.original, mutated[m.offset]);
        }
        assert_eq!(mutations.len(), 256);
    }

    #[test]
    fn mutate_payload_zero_rate_is_identity() {
        let mut rng = StdRng::seed_from_u64(1);
        let base = vec![0x00u8, 0x01, 0x02, 0x03];
        let (mutated, mutations) = mutate_payload_with(&base, 0.0, &mut rng);
        assert_eq!(mutated, base);
        assert!(mutations.is_empty());
    }

    #[test]
    fn mutate_payload_is_deterministic_with_seed() {
        let base = b"hello world".to_vec();
        let (a, _) = mutate_payload_with(&base, 0.5, &mut StdRng::seed_from_u64(7));
        let (b, _) = mutate_payload_with(&base, 0.5, &mut StdRng::seed_from_u64(7));
        assert_eq!(a, b);
    }

    #[test]
    fn clamp_fuzz_config_bounds_inputs() {
        let mut config = FuzzConfig {
            target_host: "127.0.0.1".into(),
            target_port: 1,
            protocol: "TCP".into(),
            base_payload: b"test".to_vec(),
            iterations: u32::MAX,
            mutation_rate: 5.0,
            timeout_ms: 0,
            allow_external: true,
        };
        clamp_fuzz_config(&mut config);
        assert_eq!(config.iterations, MAX_FUZZ_ITERATIONS);
        assert_eq!(config.mutation_rate, 1.0);
        assert_eq!(config.timeout_ms, 100);

        config.iterations = 0;
        config.mutation_rate = -1.0;
        config.timeout_ms = 99_999;
        clamp_fuzz_config(&mut config);
        assert_eq!(config.iterations, 0);
        assert_eq!(config.mutation_rate, 0.0);
        assert_eq!(config.timeout_ms, MAX_FUZZ_TIMEOUT_MS);
    }

    #[test]
    fn run_fuzz_respects_cancellation() {
        let cancel = Arc::new(AtomicBool::new(true));
        let results = run_fuzz(
            FuzzConfig {
                target_host: "127.0.0.1".into(),
                target_port: 1,
                protocol: "TCP".into(),
                base_payload: b"test".to_vec(),
                iterations: 1000,
                mutation_rate: 0.5,
                timeout_ms: 100,
                allow_external: true,
            },
            Some(cancel),
            None::<fn(u32, u32)>,
        );
        assert!(results.is_empty());
    }

    #[test]
    fn run_fuzz_reports_progress() {
        use std::sync::Mutex;
        let calls = Arc::new(Mutex::new(Vec::new()));
        let calls_ref = calls.clone();
        let cancel = Arc::new(AtomicBool::new(false));
        let cb = move |done: u32, total: u32| {
            calls_ref.lock().unwrap().push((done, total));
        };
        run_fuzz(
            FuzzConfig {
                target_host: "127.0.0.1".into(),
                target_port: 1,
                protocol: "TCP".into(),
                base_payload: b"".to_vec(),
                iterations: 5,
                mutation_rate: 0.5,
                timeout_ms: 100,
                allow_external: true,
            },
            Some(cancel),
            Some(cb),
        );
        let calls = calls.lock().unwrap();
        assert!(
            calls.contains(&(5, 5)),
            "final progress missing: {:?}",
            *calls
        );
    }

    #[test]
    fn is_local_address_matches_private_ranges() {
        assert!(is_local_address("127.0.0.1"));
        assert!(is_local_address("localhost"));
        assert!(is_local_address("::1"));
        assert!(is_local_address("192.168.1.10"));
        assert!(is_local_address("10.0.0.1"));
        assert!(is_local_address("172.16.0.1"));
        assert!(is_local_address("172.31.255.255"));
        assert!(!is_local_address("172.15.0.1"));
        assert!(!is_local_address("172.32.0.1"));
        assert!(!is_local_address("8.8.8.8"));
        assert!(!is_local_address("example.com"));
    }

    #[test]
    fn replay_blocks_external_targets_by_default() {
        let result = replay_packet(
            b"probe",
            ReplayConfig {
                target_host: "8.8.8.8".into(),
                target_port: 53,
                protocol: "UDP".into(),
                timeout_ms: 500,
                allow_external: false,
            },
        );
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or("").contains("Blocked"));
    }

    #[test]
    fn udp_replay_round_trips_to_local_listener() {
        let listener = UdpSocket::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        let echo = listener.try_clone().expect("clone listener");
        std::thread::spawn(move || {
            let mut buf = [0u8; 1024];
            if let Ok((n, peer)) = echo.recv_from(&mut buf) {
                let _ = echo.send_to(&buf[..n], peer);
            }
        });

        let result = replay_packet(
            b"ping",
            ReplayConfig {
                target_host: "127.0.0.1".into(),
                target_port: addr.port(),
                protocol: "UDP".into(),
                timeout_ms: 2000,
                allow_external: true,
            },
        );
        assert!(result.success, "replay failed: {:?}", result.error);
        assert_eq!(result.response.as_deref(), Some(b"ping".as_slice()));
        assert_eq!(result.bytes_sent, 4);
    }

    #[test]
    fn tcp_replay_round_trips_to_local_listener() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                if let Ok(n) = stream.read(&mut buf) {
                    let _ = stream.write_all(&buf[..n]);
                }
            }
        });

        let result = replay_packet(
            b"hello tcp",
            ReplayConfig {
                target_host: "127.0.0.1".into(),
                target_port: addr.port(),
                protocol: "TCP".into(),
                timeout_ms: 2000,
                allow_external: true,
            },
        );
        assert!(result.success, "replay failed: {:?}", result.error);
        assert_eq!(result.response.as_deref(), Some(b"hello tcp".as_slice()));
    }

    #[test]
    fn tcp_replay_timeout_yields_empty_response() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind listener");
        let addr = listener.local_addr().expect("local addr");
        std::thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf);
                std::thread::sleep(Duration::from_millis(2000));
            }
        });

        let result = replay_packet(
            b"no reply",
            ReplayConfig {
                target_host: "127.0.0.1".into(),
                target_port: addr.port(),
                protocol: "TCP".into(),
                timeout_ms: 200,
                allow_external: true,
            },
        );
        assert!(result.success);
        assert!(result.response.is_none());
    }

    #[test]
    fn connection_refused_reports_failure() {
        let result = replay_packet(
            b"x",
            ReplayConfig {
                target_host: "127.0.0.1".into(),
                target_port: 1,
                protocol: "TCP".into(),
                timeout_ms: 500,
                allow_external: true,
            },
        );
        assert!(!result.success);
        assert!(result.error.is_some());
    }
}
