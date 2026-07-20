use log::info;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::capture::CapturedPacket;

const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const PCAP_VERSION_MAJOR: u16 = 2;
const PCAP_VERSION_MINOR: u16 = 4;
const PCAP_LINKTYPE_ETHERNET: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedPacket {
    pub id: String,
    pub name: String,
    pub description: String,
    pub timestamp: String,
    pub saved_at: String,
    pub protocol: String,
    pub src_endpoint: String,
    pub dst_endpoint: String,
    pub raw_bytes: Vec<u8>,
    pub payload: Vec<u8>,
    pub payload_hex: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRecord {
    pub id: String,
    pub packet_id: Option<String>,
    pub packet_name: Option<String>,
    pub timestamp: String,
    pub target_host: String,
    pub target_port: u16,
    pub protocol: String,
    pub bytes_sent: usize,
    pub success: bool,
    pub response_bytes: Option<Vec<u8>>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedSequence {
    pub id: String,
    pub name: String,
    pub description: String,
    pub created_at: String,
    pub steps: Vec<crate::replay::SequenceStep>,
    pub tags: Vec<String>,
}

pub struct Storage {
    db: Mutex<Connection>,
    data_dir: PathBuf,
}

impl Storage {
    pub fn new(data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;

        let db_path = data_dir.join("byteplay.db");
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;

        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS saved_packets (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                timestamp TEXT NOT NULL,
                saved_at TEXT NOT NULL,
                protocol TEXT DEFAULT '',
                src_endpoint TEXT DEFAULT '',
                dst_endpoint TEXT DEFAULT '',
                raw_bytes BLOB NOT NULL,
                payload BLOB DEFAULT X'',
                payload_hex TEXT DEFAULT '',
                tags TEXT DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS replay_history (
                id TEXT PRIMARY KEY,
                packet_id TEXT,
                packet_name TEXT,
                timestamp TEXT NOT NULL,
                target_host TEXT NOT NULL,
                target_port INTEGER NOT NULL,
                protocol TEXT DEFAULT '',
                bytes_sent INTEGER DEFAULT 0,
                success INTEGER DEFAULT 0,
                response_bytes BLOB,
                error TEXT,
                duration_ms INTEGER DEFAULT 0
            );

            CREATE TABLE IF NOT EXISTS saved_sequences (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT DEFAULT '',
                created_at TEXT NOT NULL,
                steps TEXT NOT NULL,
                tags TEXT DEFAULT '[]'
            );
            ",
        )
        .map_err(|e| format!("Failed to create tables: {}", e))?;

        info!("Storage initialized at {:?}", db_path);

        Ok(Self {
            db: Mutex::new(conn),
            data_dir,
        })
    }

    pub fn save_packet(&self, packet: &SavedPacket) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT OR REPLACE INTO saved_packets (id, name, description, timestamp, saved_at, protocol, src_endpoint, dst_endpoint, raw_bytes, payload, payload_hex, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                packet.id,
                packet.name,
                packet.description,
                packet.timestamp,
                packet.saved_at,
                packet.protocol,
                packet.src_endpoint,
                packet.dst_endpoint,
                packet.raw_bytes,
                packet.payload,
                packet.payload_hex,
                serde_json::to_string(&packet.tags).unwrap_or_default(),
            ],
        ).map_err(|e| format!("Failed to save packet: {}", e))?;
        Ok(())
    }

    pub fn get_saved_packets(&self) -> Result<Vec<SavedPacket>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT id, name, description, timestamp, saved_at, protocol, src_endpoint, dst_endpoint, raw_bytes, payload, payload_hex, tags FROM saved_packets ORDER BY saved_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(11)?;
                let tags: Vec<String> =
                    serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(SavedPacket {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    timestamp: row.get(3)?,
                    saved_at: row.get(4)?,
                    protocol: row.get(5)?,
                    src_endpoint: row.get(6)?,
                    dst_endpoint: row.get(7)?,
                    raw_bytes: row.get(8)?,
                    payload: row.get(9)?,
                    payload_hex: row.get(10)?,
                    tags,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut packets = Vec::new();
        for row in rows {
            if let Ok(p) = row {
                packets.push(p);
            }
        }
        Ok(packets)
    }

    pub fn delete_saved_packet(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute("DELETE FROM saved_packets WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_replay(&self, record: &ReplayRecord) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute(
            "INSERT INTO replay_history (id, packet_id, packet_name, timestamp, target_host, target_port, protocol, bytes_sent, success, response_bytes, error, duration_ms) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                record.id,
                record.packet_id,
                record.packet_name,
                record.timestamp,
                record.target_host,
                record.target_port,
                record.protocol,
                record.bytes_sent,
                record.success as i32,
                record.response_bytes,
                record.error,
                record.duration_ms,
            ],
        ).map_err(|e| format!("Failed to record replay: {}", e))?;
        Ok(())
    }

    pub fn get_replay_history(&self) -> Result<Vec<ReplayRecord>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT id, packet_id, packet_name, timestamp, target_host, target_port, protocol, bytes_sent, success, response_bytes, error, duration_ms FROM replay_history ORDER BY timestamp DESC LIMIT {}", MAX_HISTORY_LIMIT,
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                Ok(ReplayRecord {
                    id: row.get(0)?,
                    packet_id: row.get(1)?,
                    packet_name: row.get(2)?,
                    timestamp: row.get(3)?,
                    target_host: row.get(4)?,
                    target_port: row.get(5)?,
                    protocol: row.get(6)?,
                    bytes_sent: row.get(7)?,
                    success: row.get::<_, i32>(8)? != 0,
                    response_bytes: row.get(9)?,
                    error: row.get(10)?,
                    duration_ms: row.get(11)?,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut records = Vec::new();
        for row in rows {
            if let Ok(r) = row {
                records.push(r);
            }
        }
        Ok(records)
    }

    pub fn save_sequence(&self, sequence: &SavedSequence) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let steps_json =
            serde_json::to_string(&sequence.steps).map_err(|e| e.to_string())?;
        db.execute(
            "INSERT OR REPLACE INTO saved_sequences (id, name, description, created_at, steps, tags) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                sequence.id,
                sequence.name,
                sequence.description,
                sequence.created_at,
                steps_json,
                serde_json::to_string(&sequence.tags).unwrap_or_default(),
            ],
        ).map_err(|e| format!("Failed to save sequence: {}", e))?;
        Ok(())
    }

    pub fn get_saved_sequences(&self) -> Result<Vec<SavedSequence>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut stmt = db
            .prepare(
                "SELECT id, name, description, created_at, steps, tags FROM saved_sequences ORDER BY created_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let steps_str: String = row.get(4)?;
                let steps: Vec<crate::replay::SequenceStep> =
                    serde_json::from_str(&steps_str).unwrap_or_default();
                let tags_str: String = row.get(5)?;
                let tags: Vec<String> =
                    serde_json::from_str(&tags_str).unwrap_or_default();
                Ok(SavedSequence {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: row.get(3)?,
                    steps,
                    tags,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut sequences = Vec::new();
        for row in rows {
            if let Ok(s) = row {
                sequences.push(s);
            }
        }
        Ok(sequences)
    }

    pub fn delete_sequence(&self, id: &str) -> Result<(), String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        db.execute("DELETE FROM saved_sequences WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn export_pcap(&self, packets: &[CapturedPacket], path: &str) -> Result<(), String> {
        let file = fs::File::create(path).map_err(|e| format!("Failed to create PCAP file: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        use std::io::Write;

        // PCAP global header
        let thiszone: i32 = 0;
        let sigfigs: u32 = 0;
        let snaplen: u32 = 65535;

        writer.write_all(&PCAP_MAGIC.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&PCAP_VERSION_MAJOR.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&PCAP_VERSION_MINOR.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&thiszone.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&sigfigs.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&snaplen.to_le_bytes()).map_err(|e| e.to_string())?;
        writer.write_all(&PCAP_LINKTYPE_ETHERNET.to_le_bytes()).map_err(|e| e.to_string())?;

        for packet in packets {
            let ts = chrono::DateTime::parse_from_rfc3339(&packet.timestamp)
                .unwrap_or_default();
            let tv_sec = ts.timestamp() as u32;
            let tv_usec = (ts.timestamp_subsec_micros()) as u32;

            let incl_len = packet.raw_bytes.len() as u32;

            writer.write_all(&tv_sec.to_le_bytes()).map_err(|e| e.to_string())?;
            writer.write_all(&tv_usec.to_le_bytes()).map_err(|e| e.to_string())?;
            writer.write_all(&incl_len.to_le_bytes()).map_err(|e| e.to_string())?;
            writer.write_all(&(packet.raw_bytes.len() as u32).to_le_bytes()).map_err(|e| e.to_string())?;
            writer.write_all(&packet.raw_bytes).map_err(|e| e.to_string())?;
        }

        info!("Exported {} packets to PCAP: {}", packets.len(), path);
        Ok(())
    }

    pub fn export_json(&self, packets: &[CapturedPacket], path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(packets).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| format!("Failed to write JSON file: {}", e))?;
        info!("Exported {} packets to JSON: {}", packets.len(), path);
        Ok(())
    }
}
