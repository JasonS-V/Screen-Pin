use rusqlite::{Connection, OptionalExtension, params};
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
    pub ocr_status: String,
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

        let has_status = conn
            .prepare("PRAGMA table_info(pins)")
            .map_err(|e| e.to_string())?
            .query_map([], |row| row.get::<_, String>(1))
            .map_err(|e| e.to_string())?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?
            .iter()
            .any(|column| column == "ocr_status");
        if !has_status {
            conn.execute_batch(
                "ALTER TABLE pins ADD COLUMN ocr_status TEXT NOT NULL DEFAULT 'ready';",
            )
            .map_err(|e| e.to_string())?;
        }
        // No worker survives an application restart.
        conn.execute(
            "UPDATE pins SET ocr_status = 'error' WHERE ocr_status = 'pending'",
            [],
        )
        .map_err(|e| e.to_string())?;
        // Repair the index left behind by older INSERT OR REPLACE writes.
        conn.execute("INSERT INTO pins_fts(pins_fts) VALUES('rebuild')", [])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn save_pin(&self, pin: &PinRecord) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.execute(
            "
            INSERT INTO pins (
                id, created_at, archived_at, image_path, pos_x, pos_y, width, height,
                monitor_id, raw_text, ocr_json, entity_type, is_sensitive, expires_at, is_pinned_forever, ocr_status
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
            ON CONFLICT(id) DO UPDATE SET
                raw_text = excluded.raw_text, ocr_json = excluded.ocr_json,
                entity_type = excluded.entity_type, is_sensitive = excluded.is_sensitive,
                expires_at = excluded.expires_at, ocr_status = excluded.ocr_status
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
                pin.ocr_status,
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

    pub fn get_pin(&self, pin_id: &str) -> Result<Option<PinRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        conn.query_row("SELECT id, created_at, archived_at, image_path, pos_x, pos_y, width, height,
            monitor_id, raw_text, ocr_json, entity_type, is_sensitive, expires_at, is_pinned_forever, ocr_status
            FROM pins WHERE id = ?1", [pin_id], Self::map_row)
            .optional().map_err(|e| e.to_string())
    }

    pub fn update_ocr(
        &self,
        id: &str,
        payload: Option<&crate::ocr::OcrPayload>,
    ) -> Result<(), String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        if let Some(payload) = payload {
            let kind = payload.entities.first().map(|entity| match entity {
                crate::ocr::SmartEntity::BankDigits { .. } => "BankDigits",
                crate::ocr::SmartEntity::WifiCredential { .. } => "WifiCredential",
                crate::ocr::SmartEntity::Checklist { .. } => "Checklist",
            });
            let json = serde_json::to_string(&payload.lines).map_err(|e| e.to_string())?;
            conn.execute(
                "UPDATE pins SET raw_text = ?2, ocr_json = ?3, entity_type = ?4,
                is_sensitive = ?5, expires_at = CASE WHEN ?5 THEN unixepoch() + 1800 ELSE NULL END,
                ocr_status = 'ready' WHERE id = ?1",
                params![id, payload.full_text, json, kind, payload.is_sensitive],
            )
            .map_err(|e| e.to_string())?;
        } else {
            conn.execute("UPDATE pins SET ocr_status = 'error' WHERE id = ?1", [id])
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn get_recent_pins(&self, limit: usize) -> Result<Vec<PinRecord>, String> {
        let conn = self.conn.lock().map_err(|e| e.to_string())?;
        let mut stmt = conn
            .prepare(
                "
                SELECT id, created_at, archived_at, image_path, pos_x, pos_y, width, height,
                       monitor_id, raw_text, ocr_json, entity_type, is_sensitive, expires_at, is_pinned_forever, ocr_status
                FROM pins
                ORDER BY created_at DESC
                LIMIT ?1
                ",
            )
            .map_err(|e| format!("Fallo en prepare get_recent_pins: {e}"))?;

        let rows = stmt
            .query_map(params![limit as i64], Self::map_row)
            .map_err(|e| format!("Fallo en query_map get_recent_pins: {e}"))?;

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
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
                       p.monitor_id, p.raw_text, p.ocr_json, p.entity_type, p.is_sensitive, p.expires_at, p.is_pinned_forever, p.ocr_status
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

        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())
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
            "UPDATE pins SET is_pinned_forever = ?1,
             expires_at = CASE WHEN ?1 = 0 AND is_sensitive = 1 THEN unixepoch() + 1800 ELSE expires_at END
             WHERE id = ?2",
            params![new_val, pin_id],
        )
        .map_err(|e| format!("Fallo al alternar retención: {e}"))?;

        Ok(new_val == 1)
    }

    /// Removes expired records atomically and returns their image paths for cleanup.
    pub fn purge_expired_sensitive_pins(&self) -> Result<Vec<String>, String> {
        let mut conn = self.conn.lock().map_err(|e| e.to_string())?;
        let transaction = conn.transaction().map_err(|e| e.to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let mut stmt = transaction
            .prepare(
                "
                SELECT id, image_path FROM pins
                WHERE is_sensitive = 1 AND is_pinned_forever = 0 AND expires_at <= ?1
                ",
            )
            .map_err(|e| e.to_string())?;

        let rows = stmt
            .query_map(params![now], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })
            .map_err(|e| e.to_string())?;

        let expired = rows
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| e.to_string())?;
        drop(stmt);
        transaction.execute("DELETE FROM pins WHERE is_sensitive = 1 AND is_pinned_forever = 0 AND expires_at <= ?1", [now])
            .map_err(|e| e.to_string())?;
        transaction.commit().map_err(|e| e.to_string())?;
        Ok(expired.into_iter().map(|(_, path)| path).collect())
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
            ocr_status: r.get(15)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_pin() -> PinRecord {
        PinRecord {
            id: "record-without-ocr".into(),
            created_at: 1000,
            archived_at: None,
            image_path: "record.png".into(),
            pos_x: -800.0,
            pos_y: 100.0,
            width: 300.0,
            height: 200.0,
            monitor_id: 1,
            raw_text: String::new(),
            ocr_json: String::new(),
            entity_type: None,
            is_sensitive: false,
            expires_at: None,
            is_pinned_forever: false,
            ocr_status: "pending".into(),
        }
    }

    #[test]
    fn retrieves_a_pin_by_id_without_searchable_text() {
        let db = MemoryDatabase::new_in_memory().unwrap();
        let pin = sample_pin();
        db.save_pin(&pin).unwrap();
        assert_eq!(db.get_pin(&pin.id).unwrap(), Some(pin));
        assert!(db.get_pin("missing").unwrap().is_none());
    }

    #[test]
    fn late_ocr_preserves_archiving_and_permanent_retention() {
        let db = MemoryDatabase::new_in_memory().unwrap();
        let pin = sample_pin();
        db.save_pin(&pin).unwrap();
        db.archive_pin(&pin.id).unwrap();
        db.toggle_pinned_forever(&pin.id).unwrap();
        db.update_ocr(
            &pin.id,
            Some(&crate::ocr::OcrPayload {
                pin_id: pin.id.clone(),
                lines: vec![],
                full_text: "Cuenta 123456789012".into(),
                entities: vec![crate::ocr::SmartEntity::BankDigits {
                    raw: "123456789012".into(),
                    clean_digits: "123456789012".into(),
                }],
                is_sensitive: true,
            }),
        )
        .unwrap();
        let result = db.get_pin(&pin.id).unwrap().unwrap();
        assert!(result.is_pinned_forever);
        assert!(result.archived_at.is_some());
        assert_eq!(result.pos_x, -800.0);
        assert_eq!(result.ocr_status, "ready");
        assert_eq!(db.search_pins("123456").unwrap().len(), 1);
    }

    #[test]
    fn migrates_legacy_records_and_rebuilds_search() {
        let db = MemoryDatabase::new_in_memory().unwrap();
        let mut pin = sample_pin();
        pin.raw_text = "Referencia antigua".into();
        db.save_pin(&pin).unwrap();
        {
            let conn = db.conn.lock().unwrap();
            conn.execute_batch("ALTER TABLE pins DROP COLUMN ocr_status;")
                .unwrap();
            MemoryDatabase::init_tables(&conn).unwrap();
        }
        assert_eq!(db.get_pin(&pin.id).unwrap().unwrap().ocr_status, "ready");
        assert_eq!(db.search_pins("antigua").unwrap().len(), 1);
    }

    #[test]
    fn search_accepts_punctuation_and_quotes() {
        let db = MemoryDatabase::new_in_memory().unwrap();
        for query in ["\"", "*", "-", "(OR)", "191-284", "hola \"mundo\""] {
            assert!(db.search_pins(query).is_ok(), "Invalid search: {query}");
        }
    }

    #[test]
    fn interrupted_ocr_does_not_stay_loading_after_restart() {
        let db = MemoryDatabase::new_in_memory().unwrap();
        let pin = sample_pin();
        db.save_pin(&pin).unwrap();
        MemoryDatabase::init_tables(&db.conn.lock().unwrap()).unwrap();
        assert_eq!(db.get_pin(&pin.id).unwrap().unwrap().ocr_status, "error");
    }

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
            ocr_status: "ready".into(),
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
            ocr_status: "ready".into(),
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

        // Updating OCR must remove the previous terms from the search index.
        let mut updated = pin1.clone();
        updated.raw_text = "Nuevo contenido".into();
        db.save_pin(&updated).unwrap();
        assert!(db.search_pins("191").unwrap().is_empty());
        assert_eq!(db.search_pins("Nuevo").unwrap().len(), 1);

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
            ocr_status: "ready".into(),
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
            ocr_status: "ready".into(),
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

        // Releasing permanent retention starts a fresh 30-minute period.
        assert!(!db.toggle_pinned_forever("pin-forever").unwrap());
        assert!(db.purge_expired_sensitive_pins().unwrap().is_empty());
    }
}
