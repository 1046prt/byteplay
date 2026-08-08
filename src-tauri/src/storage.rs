use log::{info, warn};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use crate::capture::CapturedPacket;

const PCAP_MAGIC: u32 = 0xa1b2c3d4;
const PCAP_VERSION_MAJOR: u16 = 2;
const PCAP_VERSION_MINOR: u16 = 4;
const PCAP_LINKTYPE_ETHERNET: u32 = 1;
const HISTORY_LIMIT: i64 = 5_000;

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
}

impl Storage {
    fn lock_db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|poisoned| {
            warn!("Storage mutex poisoned, recovering");
            poisoned.into_inner()
        })
    }

    pub fn new(data_dir: PathBuf) -> Result<Self, String> {
        fs::create_dir_all(&data_dir).map_err(|e| format!("Failed to create data dir: {}", e))?;
        let db_path = data_dir.join("byteplay.db");
        let conn =
            Connection::open(&db_path).map_err(|e| format!("Failed to open database: {}", e))?;
        Self::init(conn)
            .map_err(|e| format!("Failed to initialize database at {:?}: {}", db_path, e))
    }

    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Failed to open in-memory database: {}", e))?;
        Self::init(conn).map_err(|e| format!("Failed to initialize in-memory database: {}", e))
    }

    fn init(conn: Connection) -> Result<Self, String> {
        conn.busy_timeout(Duration::from_millis(5000))
            .map_err(|e| format!("Failed to set busy timeout: {}", e))?;
        let _ = conn.pragma_update(None, "journal_mode", "WAL");

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

        info!("Storage initialized");

        Ok(Self {
            db: Mutex::new(conn),
        })
    }

    pub fn save_packet(&self, packet: &SavedPacket) -> Result<(), String> {
        let db = self.lock_db();
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
        let db = self.lock_db();
        let mut stmt = db
            .prepare(
                "SELECT id, name, description, timestamp, saved_at, protocol, src_endpoint, dst_endpoint, raw_bytes, payload, payload_hex, tags FROM saved_packets ORDER BY saved_at DESC",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map([], |row| {
                let tags_str: String = row.get(11)?;
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
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
        for p in rows.flatten() {
            packets.push(p);
        }
        Ok(packets)
    }

    pub fn delete_saved_packet(&self, id: &str) -> Result<(), String> {
        let db = self.lock_db();
        db.execute("DELETE FROM saved_packets WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn update_saved_packet(
        &self,
        id: &str,
        name: &str,
        description: &str,
        tags: &[String],
    ) -> Result<(), String> {
        let db = self.lock_db();
        let tags_json = serde_json::to_string(tags).unwrap_or_default();
        db.execute(
            "UPDATE saved_packets SET name = ?1, description = ?2, tags = ?3 WHERE id = ?4",
            params![name, description, tags_json, id],
        )
        .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record_replay(&self, record: &ReplayRecord) -> Result<(), String> {
        let db = self.lock_db();
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

        db.execute(
            "DELETE FROM replay_history WHERE rowid IN (SELECT rowid FROM replay_history ORDER BY rowid DESC LIMIT -1 OFFSET ?1)",
            params![HISTORY_LIMIT],
        )
        .map_err(|e| format!("Failed to prune replay history: {}", e))?;

        Ok(())
    }

    pub fn get_replay_history(&self) -> Result<Vec<ReplayRecord>, String> {
        let db = self.lock_db();
        let mut stmt = db
            .prepare(
                "SELECT id, packet_id, packet_name, timestamp, target_host, target_port, protocol, bytes_sent, success, response_bytes, error, duration_ms FROM replay_history ORDER BY timestamp DESC LIMIT 500",
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
        for r in rows.flatten() {
            records.push(r);
        }
        Ok(records)
    }

    pub fn save_sequence(&self, sequence: &SavedSequence) -> Result<(), String> {
        let db = self.lock_db();
        let steps_json = serde_json::to_string(&sequence.steps).map_err(|e| e.to_string())?;
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
        let db = self.lock_db();
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
                let tags: Vec<String> = serde_json::from_str(&tags_str).unwrap_or_default();
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
        for s in rows.flatten() {
            sequences.push(s);
        }
        Ok(sequences)
    }

    pub fn delete_sequence(&self, id: &str) -> Result<(), String> {
        let db = self.lock_db();
        db.execute("DELETE FROM saved_sequences WHERE id = ?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn export_pcap(&self, packets: &[CapturedPacket], path: &str) -> Result<(), String> {
        let file =
            fs::File::create(path).map_err(|e| format!("Failed to create PCAP file: {}", e))?;
        let mut writer = std::io::BufWriter::new(file);

        use std::io::Write;

        // PCAP global header
        let thiszone: i32 = 0;
        let sigfigs: u32 = 0;
        let snaplen: u32 = 65535;

        writer
            .write_all(&PCAP_MAGIC.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&PCAP_VERSION_MAJOR.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&PCAP_VERSION_MINOR.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&thiszone.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&sigfigs.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&snaplen.to_le_bytes())
            .map_err(|e| e.to_string())?;
        writer
            .write_all(&PCAP_LINKTYPE_ETHERNET.to_le_bytes())
            .map_err(|e| e.to_string())?;

        for packet in packets {
            let ts = chrono::DateTime::parse_from_rfc3339(&packet.timestamp).unwrap_or_default();
            let tv_sec = ts.timestamp() as u32;
            let tv_usec = ts.timestamp_subsec_micros();

            let incl_len = packet.raw_bytes.len() as u32;

            writer
                .write_all(&tv_sec.to_le_bytes())
                .map_err(|e| e.to_string())?;
            writer
                .write_all(&tv_usec.to_le_bytes())
                .map_err(|e| e.to_string())?;
            writer
                .write_all(&incl_len.to_le_bytes())
                .map_err(|e| e.to_string())?;
            writer
                .write_all(&(packet.raw_bytes.len() as u32).to_le_bytes())
                .map_err(|e| e.to_string())?;
            writer
                .write_all(&packet.raw_bytes)
                .map_err(|e| e.to_string())?;
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

    pub fn import_pcap(&self, path: &str) -> Result<Vec<CapturedPacket>, String> {
        let data = fs::read(path).map_err(|e| format!("Failed to read PCAP file: {}", e))?;
        if data.len() < 24 {
            return Err("File too small to be a valid PCAP".to_string());
        }

        let magic = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
        let (big_endian, nanosecond) = match magic {
            0xa1b2c3d4 => (false, false),
            0xd4c3b2a1 => (true, false),
            0xa1b23c4d => (false, true),
            0x4d3cb2a1 => (true, true),
            _ => return Err(format!("Invalid PCAP magic: 0x{:08x}", magic)),
        };

        let read_u32 = |offset: usize| -> u32 {
            let bytes = [
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ];
            if big_endian {
                u32::from_be_bytes(bytes)
            } else {
                u32::from_le_bytes(bytes)
            }
        };

        let linktype = read_u32(20);
        let mut offset = 24usize;
        let mut packets = Vec::new();
        let mut idx = 0usize;

        while offset + 16 <= data.len() {
            let tv_sec = read_u32(offset);
            let tv_frac = read_u32(offset + 4);
            let incl_len = read_u32(offset + 8) as usize;
            let _orig_len = read_u32(offset + 12);

            offset += 16;
            if offset + incl_len > data.len() {
                break;
            }

            let raw_bytes = data[offset..offset + incl_len].to_vec();
            idx += 1;

            let usec = if nanosecond { tv_frac / 1000 } else { tv_frac };
            let ts_str = chrono::DateTime::from_timestamp(tv_sec as i64, usec * 1000)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| format!("{}.{:06}", tv_sec, usec));

            let parsed = crate::parser::parse_packet_with_linktype(
                raw_bytes,
                "imported".to_string(),
                linktype,
            );
            let captured = CapturedPacket {
                id: parsed.id,
                seq: 0,
                timestamp: ts_str,
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
                capture_index: idx,
            };

            packets.push(captured);
            offset += incl_len;
        }

        info!("Imported {} packets from PCAP: {}", packets.len(), path);
        Ok(packets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::SequenceStep;
    use tempfile::tempdir;

    fn test_storage() -> (tempfile::TempDir, Storage) {
        let dir = tempdir().expect("temp dir");
        let storage = Storage::new(dir.path().to_path_buf()).expect("storage");
        (dir, storage)
    }

    fn saved_packet(name: &str) -> SavedPacket {
        SavedPacket {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            description: "desc".to_string(),
            timestamp: "2026-01-01T00:00:00Z".to_string(),
            saved_at: "2026-01-01T00:00:00Z".to_string(),
            protocol: "TCP".to_string(),
            src_endpoint: "127.0.0.1:1".to_string(),
            dst_endpoint: "127.0.0.1:2".to_string(),
            raw_bytes: vec![1, 2, 3],
            payload: vec![4, 5, 6],
            payload_hex: "04 05 06".to_string(),
            tags: vec!["a".to_string(), "b".to_string()],
        }
    }

    fn replay_record() -> ReplayRecord {
        ReplayRecord {
            id: uuid::Uuid::new_v4().to_string(),
            packet_id: None,
            packet_name: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            target_host: "127.0.0.1".to_string(),
            target_port: 8080,
            protocol: "TCP".to_string(),
            bytes_sent: 10,
            success: true,
            response_bytes: Some(vec![1, 2]),
            error: None,
            duration_ms: 5,
        }
    }

    #[test]
    fn saved_packet_round_trip() {
        let (_dir, storage) = test_storage();
        let packet = saved_packet("http-get");
        storage.save_packet(&packet).expect("save");

        let all = storage.get_saved_packets().expect("get");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "http-get");
        assert_eq!(all[0].raw_bytes, vec![1, 2, 3]);
        assert_eq!(all[0].tags, vec!["a", "b"]);

        storage
            .update_saved_packet(&packet.id, "renamed", "new desc", &["x".to_string()])
            .expect("update");
        let updated = storage.get_saved_packets().expect("get");
        assert_eq!(updated[0].name, "renamed");
        assert_eq!(updated[0].tags, vec!["x"]);

        storage.delete_saved_packet(&packet.id).expect("delete");
        assert!(storage.get_saved_packets().expect("get").is_empty());
    }

    #[test]
    fn sequence_round_trip() {
        let (_dir, storage) = test_storage();
        let sequence = SavedSequence {
            id: uuid::Uuid::new_v4().to_string(),
            name: "seq1".to_string(),
            description: String::new(),
            created_at: chrono::Utc::now().to_rfc3339(),
            steps: vec![SequenceStep {
                name: "step1".to_string(),
                target_host: "127.0.0.1".to_string(),
                target_port: 80,
                protocol: "TCP".to_string(),
                data: vec![0xDE, 0xAD],
                delay_ms: 10,
                timeout_ms: Some(1000),
            }],
            tags: vec![],
        };
        storage.save_sequence(&sequence).expect("save");
        let all = storage.get_saved_sequences().expect("get");
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].steps[0].data, vec![0xDE, 0xAD]);
        assert_eq!(all[0].steps[0].timeout_ms, Some(1000));
    }

    #[test]
    fn replay_history_prunes_to_limit() {
        let (_dir, storage) = test_storage();
        for _ in 0..(HISTORY_LIMIT + 100) {
            storage.record_replay(&replay_record()).expect("record");
        }
        let db = storage.db.lock().unwrap();
        let count: i64 = db
            .query_row("SELECT COUNT(*) FROM replay_history", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, HISTORY_LIMIT);
    }

    #[test]
    fn recovers_from_poisoned_mutex() {
        let (_dir, storage) = test_storage();
        let shared = std::sync::Arc::new(storage);
        let handle = {
            let shared = shared.clone();
            std::thread::spawn(move || {
                let _guard = shared.db.lock().expect("lock");
                panic!("intentional panic to poison the mutex");
            })
        };
        assert!(handle.join().is_err());

        shared.record_replay(&replay_record()).expect("still works");
        assert_eq!(shared.get_replay_history().expect("get").len(), 1);
    }

    #[test]
    fn in_memory_storage_works() {
        let storage = Storage::in_memory().expect("in-memory storage");
        storage.save_packet(&saved_packet("mem")).expect("save");
        assert_eq!(storage.get_saved_packets().expect("get").len(), 1);
    }
}
