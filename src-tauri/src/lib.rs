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

    let src_endpoint = packet
        .ipv4
        .as_ref()
        .map(|ip| {
            let port = packet
                .tcp
                .as_ref()
                .map(|t| t.src_port)
                .or_else(|| packet.udp.as_ref().map(|u| u.src_port))
                .unwrap_or(0);
            format!("{}:{}", ip.src_ip, port)
        })
        .or_else(|| {
            packet.ipv6.as_ref().map(|ip| {
                let port = packet
                    .tcp
                    .as_ref()
                    .map(|t| t.src_port)
                    .or_else(|| packet.udp.as_ref().map(|u| u.src_port))
                    .unwrap_or(0);
                format!("{}:{}", ip.src_ip, port)
            })
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let dst_endpoint = packet
        .ipv4
        .as_ref()
        .map(|ip| {
            let port = packet
                .tcp
                .as_ref()
                .map(|t| t.dst_port)
                .or_else(|| packet.udp.as_ref().map(|u| u.dst_port))
                .unwrap_or(0);
            format!("{}:{}", ip.dst_ip, port)
        })
        .or_else(|| {
            packet.ipv6.as_ref().map(|ip| {
                let port = packet
                    .tcp
                    .as_ref()
                    .map(|t| t.dst_port)
                    .or_else(|| packet.udp.as_ref().map(|u| u.dst_port))
                    .unwrap_or(0);
                format!("{}:{}", ip.dst_ip, port)
            })
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
async fn replay_packet(
    state: State<'_, AppState>,
    data: Vec<u8>,
    config: ReplayConfig,
) -> Result<replay::ReplayResult, String> {
    let result = tauri::async_runtime::spawn_blocking(move || replay::replay_packet(&data, config.clone()))
        .await
        .map_err(|e| format!("Task join error: {}", e))?;

    let record = ReplayRecord {
        id: uuid::Uuid::new_v4().to_string(),
        packet_id: None,
        packet_name: None,
        timestamp: result.timestamp.clone(),
        target_host: config.target_host,
        target_port: config.target_port,
        protocol: config.protocol,
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
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            list_interfaces,
            start_capture,
            stop_capture,
            poll_packets,
            get_packet_by_id,
            clear_packets,
            save_packet,
            get_saved_packets,
            delete_saved_packet,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
