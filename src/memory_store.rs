use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MemoryEntry {
    pub id: String,
    pub category: String,
    pub content: String,
    pub confidence: f64,
    pub timestamp: u64,
}

pub struct MemoryStore {
    conn: Connection,
}

impl MemoryStore {
    pub fn new(db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS long_term_memory (
                id TEXT PRIMARY KEY,
                category TEXT NOT NULL,
                content TEXT NOT NULL,
                confidence REAL NOT NULL,
                timestamp INTEGER NOT NULL
            )",
            [],
        )?;
        Ok(Self { conn })
    }

    pub fn store(&self, entry: &MemoryEntry) -> Result<(), Box<dyn std::error::Error>> {
        self.conn.execute(
            "INSERT OR REPLACE INTO long_term_memory (id, category, content, confidence, timestamp) 
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![entry.id, entry.category, entry.content, entry.confidence, entry.timestamp],
        )?;
        Ok(())
    }

    pub fn query_by_category(&self, category: &str) -> Result<Vec<MemoryEntry>, Box<dyn std::error::Error>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, category, content, confidence, timestamp FROM long_term_memory WHERE category = ?1 ORDER BY timestamp DESC"
        )?;
        let entries = stmt.query_map(params![category], |row| {
            Ok(MemoryEntry {
                id: row.get(0)?,
                category: row.get(1)?,
                content: row.get(2)?,
                confidence: row.get(3)?,
                timestamp: row.get(4)?,
            })
        })?;
        let mut result = Vec::new();
        for entry in entries {
            result.push(entry?);
        }
        Ok(result)
    }
}
