use rusqlite::{params, Connection};

use crate::core::error::{MemAgentError, Result};
use crate::core::types::{Memory, VectorEntry};

#[derive(Debug, Clone)]
pub struct DbStats {
    pub memory_count: usize,
    pub vector_count: usize,
}

pub fn insert_memory(conn: &Connection, title: &str, content: &str, tags: &str) -> Result<i64> {
    conn.execute(
        "INSERT INTO memories (title, content, tags, updated_at) VALUES (?1, ?2, ?3, datetime('now'))",
        params![title, content, tags],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_vector(conn: &Connection, memory_id: i64, vector: &[f32]) -> Result<()> {
    let blob: &[u8] = bytemuck::cast_slice(vector);
    conn.execute(
        "INSERT OR REPLACE INTO vectors (memory_id, vector, created_at) VALUES (?1, ?2, datetime('now'))",
        params![memory_id, blob],
    )?;
    Ok(())
}

pub fn get_memory(conn: &Connection, id: i64) -> Result<Memory> {
    conn.query_row(
        "SELECT id, title, content, tags, created_at, updated_at FROM memories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                title: row.get(1)?,
                content: row.get(2)?,
                tags: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => MemAgentError::NotExist(format!("memory id={id}")),
        other => MemAgentError::Db(other),
    })
}

pub fn get_memory_with_vector(conn: &Connection, id: i64) -> Result<(Memory, Option<Vec<f32>>)> {
    let memory = get_memory(conn, id)?;

    let vector = conn
        .query_row(
            "SELECT vector FROM vectors WHERE memory_id = ?1",
            params![id],
            |row| {
                let blob: Vec<u8> = row.get(0)?;
                let floats: Vec<f32> = bytemuck::cast_slice(&blob).to_vec();
                Ok(floats)
            },
        )
        .ok();

    Ok((memory, vector))
}

pub fn list_memories(conn: &Connection, limit: usize) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id, title, content, tags, created_at, updated_at FROM memories ORDER BY updated_at DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(Memory {
            id: row.get(0)?,
            title: row.get(1)?,
            content: row.get(2)?,
            tags: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;

    let mut memories = Vec::new();
    for row in rows {
        memories.push(row?);
    }
    Ok(memories)
}

pub fn update_memory(
    conn: &Connection,
    id: i64,
    title: &str,
    content: &str,
    tags: &str,
) -> Result<()> {
    let affected = conn.execute(
        "UPDATE memories SET title = ?1, content = ?2, tags = ?3, updated_at = datetime('now') WHERE id = ?4",
        params![title, content, tags, id],
    )?;

    if affected == 0 {
        return Err(MemAgentError::NotExist(format!("memory id={id}")));
    }
    Ok(())
}

pub fn delete_memory(conn: &Connection, id: i64) -> Result<bool> {
    let affected = conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
    Ok(affected > 0)
}

pub fn load_all_vectors(conn: &Connection) -> Result<Vec<VectorEntry>> {
    let mut stmt = conn.prepare("SELECT memory_id, vector FROM vectors")?;
    let rows = stmt.query_map([], |row| {
        let memory_id: i64 = row.get(0)?;
        let blob: Vec<u8> = row.get(1)?;
        let vector: Vec<f32> = bytemuck::cast_slice(&blob).to_vec();
        Ok(VectorEntry { memory_id, vector })
    })?;

    let mut entries = Vec::new();
    for row in rows {
        entries.push(row?);
    }
    Ok(entries)
}

pub fn get_stats(conn: &Connection) -> Result<DbStats> {
    let memory_count: usize = conn
        .query_row("SELECT count(*) FROM memories", [], |row| row.get::<_, i64>(0))
        .map(|v| v as usize)?;

    let vector_count: usize = conn
        .query_row("SELECT count(*) FROM vectors", [], |row| row.get::<_, i64>(0))
        .map(|v| v as usize)?;

    Ok(DbStats {
        memory_count,
        vector_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema::init_db;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();
        conn
    }

    #[test]
    fn test_insert_and_get_memory() {
        let conn = setup();
        let id = insert_memory(&conn, "Test Title", "Test content", "rust,test").unwrap();
        assert!(id > 0);

        let mem = get_memory(&conn, id).unwrap();
        assert_eq!(mem.title, "Test Title");
        assert_eq!(mem.content, "Test content");
        assert_eq!(mem.tags, "rust,test");
    }

    #[test]
    fn test_get_memory_not_found() {
        let conn = setup();
        let err = get_memory(&conn, 999).unwrap_err();
        assert!(format!("{err}").contains("Not found"));
    }

    #[test]
    fn test_insert_and_get_vector() {
        let conn = setup();
        let id = insert_memory(&conn, "Vec Test", "content", "").unwrap();
        let vec = vec![1.0f32, 2.0, 3.0, 4.0];

        insert_vector(&conn, id, &vec).unwrap();

        let (_, got) = get_memory_with_vector(&conn, id).unwrap();
        assert_eq!(got, Some(vec![1.0f32, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_update_memory() {
        let conn = setup();
        let id = insert_memory(&conn, "Old", "old content", "").unwrap();

        update_memory(&conn, id, "New Title", "new content", "updated").unwrap();
        let mem = get_memory(&conn, id).unwrap();
        assert_eq!(mem.title, "New Title");
        assert_eq!(mem.content, "new content");
        assert_eq!(mem.tags, "updated");
    }

    #[test]
    fn test_update_memory_not_found() {
        let conn = setup();
        let err = update_memory(&conn, 999, "x", "y", "z").unwrap_err();
        assert!(format!("{err}").contains("Not found"));
    }

    #[test]
    fn test_delete_memory() {
        let conn = setup();
        let id = insert_memory(&conn, "ToDelete", "bye", "").unwrap();

        let deleted = delete_memory(&conn, id).unwrap();
        assert!(deleted);

        let err = get_memory(&conn, id).unwrap_err();
        assert!(format!("{err}").contains("Not found"));
    }

    #[test]
    fn test_delete_missing() {
        let conn = setup();
        assert!(!delete_memory(&conn, 999).unwrap());
    }

    #[test]
    fn test_list_memories() {
        let conn = setup();
        insert_memory(&conn, "A", "first", "").unwrap();
        insert_memory(&conn, "B", "second", "").unwrap();
        insert_memory(&conn, "C", "third", "").unwrap();

        let list = list_memories(&conn, 5).unwrap();
        assert_eq!(list.len(), 3);
    }

    #[test]
    fn test_load_all_vectors() {
        let conn = setup();
        let id1 = insert_memory(&conn, "V1", "c1", "").unwrap();
        let id2 = insert_memory(&conn, "V2", "c2", "").unwrap();
        insert_vector(&conn, id1, &[1.0, 2.0]).unwrap();
        insert_vector(&conn, id2, &[3.0, 4.0]).unwrap();

        let entries = load_all_vectors(&conn).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_stats() {
        let conn = setup();
        let id = insert_memory(&conn, "Stats", "test", "").unwrap();
        insert_vector(&conn, id, &[1.0]).unwrap();

        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.memory_count, 1);
        assert_eq!(stats.vector_count, 1);
    }
}
