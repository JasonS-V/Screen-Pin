use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PinRecord {
    pub id: String,
    pub created_at: i64,
    pub archived_at: Option<i64>,
    pub image_path: String,
    pub pos_x: f64,
    pub pos_y: f64,
    pub width: f64,
    pub height: f64,
    pub monitor_id: u32,
    pub raw_text: String,
    pub ocr_json: String,
    pub entity_type: Option<String>,
    pub is_sensitive: bool,
    pub expires_at: Option<i64>,
    pub is_pinned_forever: bool,
}

pub struct MemoryDatabase {
    conn: Arc<Mutex<Connection>>,
    pub data_dir: PathBuf,
}

impl MemoryDatabase {
    pub fn new(data_dir: PathBuf) -> Result<Self, String> {
        std::fs::create_dir_all(&data_dir)
            .map_err(|e| format!("Fallo al crear directorio de datos: {e}"))?;

        let db_path = data_dir.join("pins.db");
        let conn = Connection::open(&db_path)
            .map_err(|e| format!("Fallo al abrir SQLite en {:?}: {e}", db_path))?;

        Self::init_tables(&conn)?;

        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            data_dir,
        })
    }

    pub fn new_in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory()
            .map_err(|e| format!("Fallo al abrir SQLite en memoria: {e}"))?;
        Self::init_tables(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
            data_dir: PathBuf::from("."),
        })
    }

    fn init_tables(conn: &Connection) -> Result<(), String> {
        conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS pins (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL,
                archived_at INTEGER,
                image_path TEXT NOT NULL,
                pos_x REAL NOT NULL,
                pos_y REAL NOT NULL,
                width REAL NOT NULL,
                height REAL NOT NULL,
                monitor_id INTEGER NOT NULL,
                raw_text TEXT NOT NULL,
                ocr_json TEXT NOT NULL,
                entity_type TEXT,
                is_sensitive INTEGER NOT NULL DEFAULT 0,
                expires_at INTEGER,
                is_pinned_forever INTEGER NOT NULL DEFAULT 0
            );

            CREATE VIRTUAL TABLE IF NOT EXISTS pins_fts USING fts5(
                id UNINDEXED,
                raw_text,
                entity_type,
                content='pins',
                content_rowid='rowid'
            );

            CREATE TRIGGER IF NOT EXISTS pins_ai AFTER INSERT ON pins BEGIN
                INSERT INTO pins_fts(rowid, id, raw_text, entity_type) 
                VALUES (new.rowid, new.id, new.raw_text, new.entity_type);
            END;

            CREATE TRIGGER IF NOT EXISTS pins_ad AFTER DELETE ON pins BEGIN
                INSERT INTO pins_fts(pins_fts, rowid, id, raw_text, entity_type) 
                VALUES('delete', old.rowid, old.id, old.raw_text, old.entity_type);
            END;

            CREATE TRIGGER IF NOT EXISTS pins_au AFTER UPDATE ON pins BEGIN
                INSERT INTO pins_fts(pins_fts, rowid, id, raw_text, entity_type) 
                VALUES('delete', old.rowid, old.id, old.raw_text, old.entity_type);
                INSERT INTO pins_fts(rowid, id, raw_text, entity_type) 
                VALUES (new.rowid, new.id, new.raw_text, new.entity_type);
            END;
            ",
        )
        .map_err(|e| format!("Fallo al inicializar tablas y FTS5: {e}"))?;

        Ok(())
    }

    pub fn save_pin(&self, pin: &PinRecord) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "
            INSERT OR REPLACE INTO pins (
                id, created_at, archived_at, image_path, pos_x, pos_y, width, height,
                monitor_id, raw_text, ocr_json, entity_type, is_sensitive, expires_at, is_pinned_forever
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)
            ",
            params![
                pin.id,
                pin.created_at,
                pin.archived_at,
                pin.image_path,
                pin.pos_x,
                pin.pos_y,
                pin.width,
                pin.height,
                pin.monitor_id,
                pin.raw_text,
                pin.ocr_json,
                pin.entity_type,
                if pin.is_sensitive { 1 } else { 0 },
                pin.expires_at,
                if pin.is_pinned_forever { 1 } else { 0 },
            ],
        )
        .map_err(|e| format!("Fallo al guardar pin: {e}"))?;

        Ok(())
    }

    pub fn archive_pin(&self, pin_id: &str) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        conn.execute(
            "UPDATE pins SET archived_at = ?1 WHERE id = ?2",
            params![now, pin_id],
        )
        .map_err(|e| format!("Fallo al archivar pin: {e}"))?;

        Ok(())
    }

    pub fn get_recent_pins(&self, limit: usize) -> Result<Vec<PinRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, created_at, archived_at, image_path, pos_x, pos_y, width, height,
                       monitor_id, raw_text, ocr_json, entity_type, is_sensitive, expires_at, is_pinned_forever
                FROM pins
                ORDER BY created_at DESC
                LIMIT ?1
                ",
            )
            .map_err(|e| format!("Fallo en prepare get_recent_pins: {e}"))?;

        let rows = stmt
            .query_map(params![limit as i64], Self::map_row)
            .map_err(|e| format!("Fallo en query_map get_recent_pins: {e}"))?;

        let mut pins = Vec::new();
        for r in rows {
            if let Ok(pin) = r {
                pins.push(pin);
            }
        }

        Ok(pins)
    }

    pub fn search_pins(&self, query: &str) -> Result<Vec<PinRecord>, String> {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return self.get_recent_pins(50);
        }

        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        // Preparar sintaxis de consulta FTS5 con prefijos para búsqueda instantánea
        let fts_query = format!("\"{}\"*", trimmed.replace('\"', "\"\""));

        let mut stmt = conn
            .prepare(
                "
                SELECT p.id, p.created_at, p.archived_at, p.image_path, p.pos_x, p.pos_y, p.width, p.height,
                       p.monitor_id, p.raw_text, p.ocr_json, p.entity_type, p.is_sensitive, p.expires_at, p.is_pinned_forever
                FROM pins p
                JOIN pins_fts ON pins_fts.id = p.id
                WHERE pins_fts MATCH ?1
                ORDER BY p.created_at DESC
                LIMIT 50
                ",
            )
            .map_err(|e| format!("Fallo en prepare search_pins: {e}"))?;

        let rows = stmt
            .query_map(params![fts_query], Self::map_row)
            .map_err(|e| format!("Fallo en query_map search_pins: {e}"))?;

        let mut pins = Vec::new();
        for r in rows {
            if let Ok(pin) = r {
                pins.push(pin);
            }
        }

        Ok(pins)
    }

    pub fn toggle_pinned_forever(&self, pin_id: &str) -> Result<bool, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let current: i32 = conn
            .query_row(
                "SELECT is_pinned_forever FROM pins WHERE id = ?1",
                params![pin_id],
                |r| r.get(0),
            )
            .map_err(|e| format!("Pin no encontrado: {e}"))?;

        let new_val = if current == 1 { 0 } else { 1 };
        conn.execute(
            "UPDATE pins SET is_pinned_forever = ?1 WHERE id = ?2",
            params![new_val, pin_id],
        )
        .map_err(|e| format!("Fallo al alternar retención: {e}"))?;

        Ok(new_val == 1)
    }

    /// Purga automática de recortes sensibles expirados (Demoniaco GC)
    pub fn purge_expired_sensitive_pins(&self) -> Result<Vec<String>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut stmt = conn
            .prepare(
                "
                SELECT id, image_path FROM pins
                WHERE is_sensitive = 1 AND is_pinned_forever = 0 AND expires_at <= ?1
                ",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![now], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
            .map_err(|e| e.to_string())?;

        let mut paths_to_delete = Vec::new();
        let mut ids_to_delete = Vec::new();

        for r in rows.flatten() {
            ids_to_delete.push(r.0);
            paths_to_delete.push(r.1);
        }

        for id in ids_to_delete {
            let _ = conn.execute("DELETE FROM pins WHERE id = ?1", params![id]);
        }

        Ok(paths_to_delete)
    }

    fn map_row(r: &rusqlite::Row) -> rusqlite::Result<PinRecord> {
        Ok(PinRecord {
            id: r.get(0)?,
            created_at: r.get(1)?,
            archived_at: r.get(2)?,
            image_path: r.get(3)?,
            pos_x: r.get(4)?,
            pos_y: r.get(5)?,
            width: r.get(6)?,
            height: r.get(7)?,
            monitor_id: r.get(8)?,
            raw_text: r.get(9)?,
            ocr_json: r.get(10)?,
            entity_type: r.get(11)?,
            is_sensitive: r.get::<_, i32>(12)? == 1,
            expires_at: r.get(13)?,
            is_pinned_forever: r.get::<_, i32>(14)? == 1,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_db_insert_and_fts_search() {
        let db = MemoryDatabase::new_in_memory().unwrap();

        let pin1 = PinRecord {
            id: "pin-bank-1".to_string(),
            created_at: 1000,
            archived_at: None,
            image_path: "path/1.webp".to_string(),
            pos_x: 100.0,
            pos_y: 100.0,
            width: 200.0,
            height: 100.0,
            monitor_id: 0,
            raw_text: "Banco Nacional Cuenta 191-2849102-0-94 Carlos Mendoza".to_string(),
            ocr_json: "{}".to_string(),
            entity_type: Some("BankDigits".to_string()),
            is_sensitive: true,
            expires_at: Some(2800),
            is_pinned_forever: false,
        };

        let pin2 = PinRecord {
            id: "pin-wifi-2".to_string(),
            created_at: 1050,
            archived_at: None,
            image_path: "path/2.webp".to_string(),
            pos_x: 400.0,
            pos_y: 200.0,
            width: 300.0,
            height: 150.0,
            monitor_id: 0,
            raw_text: "Red Invitados Password Secreto2026".to_string(),
            ocr_json: "{}".to_string(),
            entity_type: Some("WifiCredential".to_string()),
            is_sensitive: true,
            expires_at: Some(2850),
            is_pinned_forever: false,
        };

        db.save_pin(&pin1).unwrap();
        db.save_pin(&pin2).unwrap();

        // Búsqueda FTS5 por fragmento de número de cuenta
        let results = db.search_pins("191").unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "pin-bank-1");

        // Búsqueda FTS5 por clave Wi-Fi
        let wifi_results = db.search_pins("Password").unwrap();
        assert_eq!(wifi_results.len(), 1);
        assert_eq!(wifi_results[0].id, "pin-wifi-2");

        // Búsqueda sin coincidencias
        let no_results = db.search_pins("inexistente").unwrap();
        assert_eq!(no_results.len(), 0);
    }

    #[test]
    fn test_purge_expired_sensitive_pins() {
        let db = MemoryDatabase::new_in_memory().unwrap();

        let expired_pin = PinRecord {
            id: "pin-expired".to_string(),
            created_at: 100,
            archived_at: None,
            image_path: "path/expired.webp".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
            width: 100.0,
            height: 100.0,
            monitor_id: 0,
            raw_text: "Cuenta 123456789012".to_string(),
            ocr_json: "{}".to_string(),
            entity_type: Some("BankDigits".to_string()),
            is_sensitive: true,
            expires_at: Some(200), // Expiró en t=200
            is_pinned_forever: false,
        };

        let forever_pin = PinRecord {
            id: "pin-forever".to_string(),
            created_at: 100,
            archived_at: None,
            image_path: "path/forever.webp".to_string(),
            pos_x: 0.0,
            pos_y: 0.0,
            width: 100.0,
            height: 100.0,
            monitor_id: 0,
            raw_text: "Cuenta 987654321098".to_string(),
            ocr_json: "{}".to_string(),
            entity_type: Some("BankDigits".to_string()),
            is_sensitive: true,
            expires_at: Some(200),
            is_pinned_forever: true, // Usuario fijó retención permanente
        };

        db.save_pin(&expired_pin).unwrap();
        db.save_pin(&forever_pin).unwrap();

        let purged = db.purge_expired_sensitive_pins().unwrap();
        assert_eq!(purged, vec!["path/expired.webp"]);

        let recent = db.get_recent_pins(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].id, "pin-forever");
    }
}
