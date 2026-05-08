use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Produit {
    pub id: Option<i64>,
    pub nom: String,
    pub type_produit: String,
    pub composition: String,
    pub doses: Vec<DoseCulture>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DoseCulture {
    pub culture: String,
    pub dose_kg_ha: f64,
    pub concentration_pct: f64,
    pub notes: String,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> Result<Self, String> {
        let db_path = Self::db_path();
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let conn = Connection::open(&db_path).map_err(|e| e.to_string())?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    fn db_path() -> PathBuf {
        if let Some(data_dir) = dirs::data_dir() {
            data_dir.join("pulve-calc").join("pulve.db")
        } else {
            PathBuf::from("pulve.db")
        }
    }

    fn init_schema(&self) -> Result<(), String> {
        self.conn
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS produits (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    nom TEXT NOT NULL,
                    type_produit TEXT NOT NULL DEFAULT '',
                    composition TEXT NOT NULL DEFAULT '',
                    doses_json TEXT NOT NULL DEFAULT '[]'
                );",
            )
            .map_err(|e| e.to_string())
    }

    pub fn list_produits(&self) -> Result<Vec<Produit>, String> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, nom, type_produit, composition, doses_json FROM produits")
            .map_err(|e| e.to_string())?;
        let rows = stmt
            .query_map([], |row| {
                let doses_json: String = row.get(4)?;
                let doses: Vec<DoseCulture> =
                    serde_json::from_str(&doses_json).unwrap_or_default();
                Ok(Produit {
                    id: Some(row.get(0)?),
                    nom: row.get(1)?,
                    type_produit: row.get(2)?,
                    composition: row.get(3)?,
                    doses,
                })
            })
            .map_err(|e| e.to_string())?;

        let mut produits = Vec::new();
        for row in rows {
            produits.push(row.map_err(|e| e.to_string())?);
        }
        Ok(produits)
    }

    pub fn insert_produit(&self, p: &Produit) -> Result<i64, String> {
        let doses_json = serde_json::to_string(&p.doses).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "INSERT INTO produits (nom, type_produit, composition, doses_json) VALUES (?1, ?2, ?3, ?4)",
                params![p.nom, p.type_produit, p.composition, doses_json],
            )
            .map_err(|e| e.to_string())?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn update_produit(&self, p: &Produit) -> Result<(), String> {
        let id = p.id.ok_or("Produit sans ID")?;
        let doses_json = serde_json::to_string(&p.doses).map_err(|e| e.to_string())?;
        self.conn
            .execute(
                "UPDATE produits SET nom=?1, type_produit=?2, composition=?3, doses_json=?4 WHERE id=?5",
                params![p.nom, p.type_produit, p.composition, doses_json, id],
            )
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn delete_produit(&self, id: i64) -> Result<(), String> {
        self.conn
            .execute("DELETE FROM produits WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}
