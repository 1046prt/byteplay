mod capture;
mod parser;
mod replay;
mod storage;

use capture::{CaptureConfig, CaptureEngine, CapturedPacket};
use replay::{execute_sequence, FuzzConfig, ReplayConfig, SequenceStep};
use storage::{ReplayRecord, SavedPacket, SavedSequence, Storage};
use std::sync::mpsc;
use std::sync::Mutex;
use tauri::State;

pub struct AppState {
    pub capture_engine: CaptureEngine,
    pub storage: Mutex<Storage>,
    pub capture_rx: Mutex<Option<mpsc::Receiver<CapturedPacket>>>,
}

#[tauri::command]
fn list_interfaces() -> Result<Vec<String>, String> {
    Ok(CaptureEngine::list_interfaces())
}

#[tauri::command]
fn start_capture(
    state: State<'_, AppState>,
    config: CaptureConfig,
) -> Result<String, String> {
    let (tx, rx) = mpsc::channel::<CapturedPacket>();
    state
        .capture_engine
        .start_capture(config.clone(), tx)?;

    *state.capture_rx.lock().map_err(|e| e.to_string())? = Some(rx);

    Ok(format!("Capture started on {}", config.interface_name))
}

#[tauri::command]
fn stop_capture(state: State<'_, AppState>) -> Result<String, String> {
    state.capture_engine.stop_capture();
    *state.capture_rx.lock().map_err(|e| e.to_string())? = None;
    Ok("Capture stopped".to_string())
}

#[tauri::command]
fn poll_packets(state: State<'_, AppState>) -> Result<Vec<CapturedPacket>, String> {
    let rx_lock = state.capture_rx.lock().map_err(|e| e.to_string())?;
    if let Some(ref rx) = *rx_lock {
        let mut packets = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(packet) => {
                    state.capture_engine.store_packet(packet.clone());
                    packets.push(packet);
                }
                Err(mpsc::TryRecvError::Empty) => break,
                Err(mpsc::TryRecvError::Disconnected) => break,
            }
        }
        Ok(packets)
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
fn get_packet_by_id(
    state: State<'_, AppState>,
    id: String,
) -> Result<Option<CapturedPacket>, String> {
    Ok(state.capture_engine.get_packet_by_id(&id))
}

#[tauri::command]
fn reparse_packet(raw_bytes: Vec<u8>, interface: String) -> Result<CapturedPacket, String> {
    use crate::capture::CapturedPacket as CP;
    use crate::parser::parse_packet;
    let parsed = parse_packet(raw_bytes, interface);
    Ok(CP {
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
        capture_index: 0,
    })
}

#[tauri::command]
fn clear_packets(state: State<'_, AppState>) -> Result<String, String> {
    state.capture_engine.clear_packets();
    Ok("Packets cleared".to_string())
}

#[tauri::command]
fn save_packet(
    state: State<'_, AppState>,
    packet_id: String,
    name: String,
    description: String,
    tags: Vec<String>,
) -> Result<SavedPacket, String> {
    let packet = state
        .capture_engine
        .get_packet_by_id(&packet_id)
        .ok_or_else(|| format!("Packet {} not found", packet_id))?;

    let protocol = packet
        .tcp
        .as_ref()
        .map(|_| "TCP".to_string())
        .or_else(|| packet.udp.as_ref().map(|_| "UDP".to_string()))
        .unwrap_or_else(|| "Unknown".to_string());

    let port = packet
        .tcp
        .as_ref()
        .map(|t| (t.src_port, t.dst_port))
        .or_else(|| packet.udp.as_ref().map(|u| (u.src_port, u.dst_port)))
        .unwrap_or((0, 0));

    let src_endpoint = packet
        .ipv4
        .as_ref()
        .map(|ip| format!("{}:{}", ip.src_ip, port.0))
        .or_else(|| {
            packet
                .ipv6
                .as_ref()
                .map(|ip| format!("{}:{}", ip.src_ip, port.0))
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let dst_endpoint = packet
        .ipv4
        .as_ref()
        .map(|ip| format!("{}:{}", ip.dst_ip, port.1))
        .or_else(|| {
            packet
                .ipv6
                .as_ref()
                .map(|ip| format!("{}:{}", ip.dst_ip, port.1))
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let saved = SavedPacket {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        timestamp: packet.timestamp.clone(),
        saved_at: chrono::Utc::now().to_rfc3339(),
        protocol,
        src_endpoint,
        dst_endpoint,
        raw_bytes: packet.raw_bytes.clone(),
        payload: packet.payload.clone(),
        payload_hex: packet.payload_hex.clone(),
        tags,
    };

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.save_packet(&saved)?;
    Ok(saved)
}

#[tauri::command]
fn get_saved_packets(state: State<'_, AppState>) -> Result<Vec<SavedPacket>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_saved_packets()
}

#[tauri::command]
fn delete_saved_packet(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.delete_saved_packet(&id)?;
    Ok("Packet deleted".to_string())
}

#[tauri::command]
fn update_saved_packet(
    state: State<'_, AppState>,
    id: String,
    name: String,
    description: String,
    tags: Vec<String>,
) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.update_saved_packet(&id, &name, &description, &tags)?;
    Ok("Packet updated".to_string())
}

#[tauri::command]
async fn replay_packet(
    state: State<'_, AppState>,
    data: Vec<u8>,
    config: ReplayConfig,
) -> Result<replay::ReplayResult, String> {
    let record_target_host = config.target_host.clone();
    let record_target_port = config.target_port;
    let record_protocol = config.protocol.clone();

    let result = replay::replay_packet(&data, config);

    let record = ReplayRecord {
        id: uuid::Uuid::new_v4().to_string(),
        packet_id: None,
        packet_name: None,
        timestamp: result.timestamp.clone(),
        target_host: record_target_host,
        target_port: record_target_port,
        protocol: record_protocol,
        bytes_sent: result.bytes_sent,
        success: result.success,
        response_bytes: result.response.clone(),
        error: result.error.clone(),
        duration_ms: result.duration_ms,
    };

    if let Ok(storage) = state.storage.lock() {
        let _ = storage.record_replay(&record);
    }

    Ok(result)
}

#[tauri::command]
fn get_replay_history(state: State<'_, AppState>) -> Result<Vec<ReplayRecord>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_replay_history()
}

#[tauri::command]
async fn execute_replay_sequence(
    state: State<'_, AppState>,
    steps: Vec<SequenceStep>,
    allow_external: bool,
) -> Result<Vec<replay::ReplayResult>, String> {
    let results = tauri::async_runtime::spawn_blocking(move || {
        execute_sequence(steps, allow_external)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    if let Ok(storage) = state.storage.lock() {
        for result in &results {
            let record = ReplayRecord {
                id: uuid::Uuid::new_v4().to_string(),
                packet_id: None,
                packet_name: None,
                timestamp: result.timestamp.clone(),
                target_host: result.target.clone(),
                target_port: 0,
                protocol: String::new(),
                bytes_sent: result.bytes_sent,
                success: result.success,
                response_bytes: result.response.clone(),
                error: result.error.clone(),
                duration_ms: result.duration_ms,
            };
            let _ = storage.record_replay(&record);
        }
    }

    Ok(results)
}

#[tauri::command]
async fn run_fuzzer(
    state: State<'_, AppState>,
    config: FuzzConfig,
) -> Result<Vec<replay::FuzzResult>, String> {
    let results = tauri::async_runtime::spawn_blocking(move || {
        replay::run_fuzz(config, None)
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?;

    if let Ok(storage) = state.storage.lock() {
        for result in &results {
            let record = ReplayRecord {
                id: uuid::Uuid::new_v4().to_string(),
                packet_id: None,
                packet_name: None,
                timestamp: result.replay_result.timestamp.clone(),
                target_host: result.replay_result.target.clone(),
                target_port: 0,
                protocol: String::new(),
                bytes_sent: result.replay_result.bytes_sent,
                success: result.replay_result.success,
                response_bytes: result.replay_result.response.clone(),
                error: result.replay_result.error.clone(),
                duration_ms: result.replay_result.duration_ms,
            };
            let _ = storage.record_replay(&record);
        }
    }

    Ok(results)
}

#[tauri::command]
fn save_sequence(
    state: State<'_, AppState>,
    name: String,
    description: String,
    steps: Vec<SequenceStep>,
    tags: Vec<String>,
) -> Result<SavedSequence, String> {
    let sequence = SavedSequence {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        description,
        created_at: chrono::Utc::now().to_rfc3339(),
        steps,
        tags,
    };

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.save_sequence(&sequence)?;
    Ok(sequence)
}

#[tauri::command]
fn get_saved_sequences(state: State<'_, AppState>) -> Result<Vec<SavedSequence>, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.get_saved_sequences()
}

#[tauri::command]
fn delete_sequence(
    state: State<'_, AppState>,
    id: String,
) -> Result<String, String> {
    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.delete_sequence(&id)?;
    Ok("Sequence deleted".to_string())
}

#[tauri::command]
fn export_pcap(
    state: State<'_, AppState>,
    packet_ids: Vec<String>,
    path: String,
) -> Result<String, String> {
    let packets: Vec<CapturedPacket> = packet_ids
        .iter()
        .filter_map(|id| state.capture_engine.get_packet_by_id(id))
        .collect();

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.export_pcap(&packets, &path)?;
    Ok(format!("Exported {} packets to {}", packets.len(), path))
}

#[tauri::command]
fn export_json(
    state: State<'_, AppState>,
    packet_ids: Vec<String>,
    path: String,
) -> Result<String, String> {
    let packets: Vec<CapturedPacket> = packet_ids
        .iter()
        .filter_map(|id| state.capture_engine.get_packet_by_id(id))
        .collect();

    let storage = state.storage.lock().map_err(|e| e.to_string())?;
    storage.export_json(&packets, &path)?;
    Ok(format!("Exported {} packets to {}", packets.len(), path))
}

#[tauri::command]
fn compute_hex_diff(
    original: Vec<u8>,
    modified: Vec<u8>,
) -> Vec<HexDiffEntry> {
    let max_len = original.len().max(modified.len());
    let mut entries = Vec::new();

    for i in 0..max_len {
        let orig_byte = original.get(i).copied();
        let mod_byte = modified.get(i).copied();
        let changed = orig_byte != mod_byte;

        entries.push(HexDiffEntry {
            offset: i,
            original: orig_byte,
            modified: mod_byte,
            changed,
        });
    }

    entries
}

#[derive(serde::Serialize, Clone)]
pub struct HexDiffEntry {
    pub offset: usize,
    pub original: Option<u8>,
    pub modified: Option<u8>,
    pub changed: bool,
}

#[tauri::command]
fn import_pcap(state: State<'_, AppState>, path: String) -> Result<Vec<CapturedPacket>, String> {
    let packets = {
        let storage = state.storage.lock().map_err(|e| e.to_string())?;
        storage.import_pcap(&path)?
    };
    for p in &packets {
        state.capture_engine.store_packet(p.clone());
    }
    Ok(packets)
}

#[derive(serde::Serialize, Clone)]
pub struct ProtocolStat {
    pub protocol: String,
    pub count: usize,
    pub bytes: usize,
}

#[derive(serde::Serialize, Clone)]
pub struct EndpointStat {
    pub endpoint: String,
    pub count: usize,
}

#[derive(serde::Serialize, Clone)]
pub struct TimeBucket {
    pub timestamp: String,
    pub count: usize,
    pub bytes: usize,
}

#[derive(serde::Serialize, Clone)]
pub struct CaptureStatsData {
    pub total_packets: usize,
    pub total_bytes: usize,
    pub protocols: Vec<ProtocolStat>,
    pub top_sources: Vec<EndpointStat>,
    pub top_destinations: Vec<EndpointStat>,
    pub timeline: Vec<TimeBucket>,
}

#[tauri::command]
fn get_capture_stats(state: State<'_, AppState>) -> Result<CaptureStatsData, String> {
    let packets = state.capture_engine.get_packets();

    let mut proto_map: std::collections::HashMap<String, (usize, usize)> = std::collections::HashMap::new();
    let mut src_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut dst_map: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_bytes: usize = 0;

    for p in &packets {
        let proto = if p.tcp.is_some() { "TCP".to_string() }
            else if p.udp.is_some() { "UDP".to_string() }
            else if let Some(ref ip4) = p.ipv4 { ip4.protocol.clone() }
            else if let Some(ref ip6) = p.ipv6 { ip6.next_header.clone() }
            else { "Other".to_string() };

        let entry = proto_map.entry(proto).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += p.frame_length;
        total_bytes += p.frame_length;

        let src_ip = p.ipv4.as_ref().map(|ip| ip.src_ip.clone())
            .or_else(|| p.ipv6.as_ref().map(|ip| ip.src_ip.clone()))
            .unwrap_or_default();
        let src_port = p.tcp.as_ref().map(|t| t.src_port)
            .or_else(|| p.udp.as_ref().map(|u| u.src_port));
        let src_ep = if let Some(port) = src_port {
            format!("{}:{}", src_ip, port)
        } else { src_ip };
        if !src_ep.is_empty() {
            *src_map.entry(src_ep).or_insert(0) += 1;
        }

        let dst_ip = p.ipv4.as_ref().map(|ip| ip.dst_ip.clone())
            .or_else(|| p.ipv6.as_ref().map(|ip| ip.dst_ip.clone()))
            .unwrap_or_default();
        let dst_port = p.tcp.as_ref().map(|t| t.dst_port)
            .or_else(|| p.udp.as_ref().map(|u| u.dst_port));
        let dst_ep = if let Some(port) = dst_port {
            format!("{}:{}", dst_ip, port)
        } else { dst_ip };
        if !dst_ep.is_empty() {
            *dst_map.entry(dst_ep).or_insert(0) += 1;
        }
    }

    let mut protocols: Vec<ProtocolStat> = proto_map.into_iter()
        .map(|(protocol, (count, bytes))| ProtocolStat { protocol, count, bytes })
        .collect();
    protocols.sort_by(|a, b| b.count.cmp(&a.count));

    let mut top_sources: Vec<EndpointStat> = src_map.into_iter()
        .map(|(endpoint, count)| EndpointStat { endpoint, count })
        .collect();
    top_sources.sort_by(|a, b| b.count.cmp(&a.count));
    top_sources.truncate(20);

    let mut top_destinations: Vec<EndpointStat> = dst_map.into_iter()
        .map(|(endpoint, count)| EndpointStat { endpoint, count })
        .collect();
    top_destinations.sort_by(|a, b| b.count.cmp(&a.count));
    top_destinations.truncate(20);

    let timeline = if packets.is_empty() {
        Vec::new()
    } else {
        let mut buckets: std::collections::BTreeMap<String, (usize, usize)> = std::collections::BTreeMap::new();
        for p in &packets {
            let minute = if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&p.timestamp) {
                dt.format("%H:%M").to_string()
            } else {
                "??".to_string()
            };
            let entry = buckets.entry(minute).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += p.frame_length;
        }
        buckets.into_iter()
            .map(|(ts, (count, bytes))| TimeBucket { timestamp: ts, count, bytes })
            .collect()
    };

    Ok(CaptureStatsData {
        total_packets: packets.len(),
        total_bytes,
        protocols,
        top_sources,
        top_destinations,
        timeline,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let data_dir = dirs_next::data_local_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("byteplay");

    let storage = Storage::new(data_dir).expect("Failed to initialize storage");

    let state = AppState {
        capture_engine: CaptureEngine::new(),
        storage: Mutex::new(storage),
        capture_rx: Mutex::new(None),
    };

    tauri::Builder::default()
        .setup(|app| {
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .build(),
            )?;
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_interfaces,
            start_capture,
            stop_capture,
            poll_packets,
            get_packet_by_id,
            reparse_packet,
            clear_packets,
            save_packet,
            get_saved_packets,
            delete_saved_packet,
            update_saved_packet,
            replay_packet,
            get_replay_history,
            execute_replay_sequence,
            run_fuzzer,
            save_sequence,
            get_saved_sequences,
            delete_sequence,
            export_pcap,
            export_json,
            compute_hex_diff,
            import_pcap,
            get_capture_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
