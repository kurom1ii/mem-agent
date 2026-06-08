use rusqlite::{params, Connection};

use crate::core::error::{MemAgentError, Result};
use crate::core::types::{Memory, Session, VectorEntry};

#[derive(Debug, Clone)]
pub struct DbStats {
    pub memory_count: usize,
    pub vector_count: usize,
    pub session_count: usize,
}

pub fn get_epoch_now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

pub fn ensure_session(conn: &Connection, id: &str, project: &str, source: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO sessions (id, project, source, title, updated_at_epoch)
         VALUES (?1, ?2, ?3, '', ?4)",
        params![id, project, source, get_epoch_now()],
    )?;
    Ok(())
}

pub fn update_session_title(conn: &Connection, id: &str, title: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET title = ?1, updated_at_epoch = ?2 WHERE id = ?3",
        params![title, get_epoch_now(), id],
    )?;
    Ok(())
}

pub fn update_session_status(conn: &Connection, id: &str, status: &str) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET status = ?1, updated_at_epoch = ?2 WHERE id = ?3",
        params![status, get_epoch_now(), id],
    )?;
    Ok(())
}

pub fn get_session(conn: &Connection, id: &str) -> Result<Session> {
    conn.query_row(
        "SELECT id, project, source, title, status, created_at_epoch, updated_at_epoch
         FROM sessions WHERE id = ?1",
        params![id],
        |row| {
            Ok(Session {
                id: row.get(0)?,
                project: row.get(1)?,
                source: row.get(2)?,
                title: row.get(3)?,
                status: row.get(4)?,
                created_at_epoch: row.get(5)?,
                updated_at_epoch: row.get(6)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            MemAgentError::NotExist(format!("session id={id}"))
        }
        other => MemAgentError::Db(other),
    })
}

pub fn insert_memory(
    conn: &Connection,
    kind: &str,
    observation_type: &str,
    session_id: Option<&str>,
    source: &str,
    project: &str,
    title: &str,
    content: &str,
    tags: &str,
    facts: &str,
    concepts: &str,
    files_read: &str,
    files_modified: &str,
    narrative: Option<&str>,
) -> Result<i64> {
    let now = get_epoch_now();
    conn.execute(
        "INSERT INTO memories (kind, observation_type, session_id, source, project,
         title, content, tags, facts, concepts, files_read, files_modified, narrative,
         created_at_epoch, updated_at_epoch)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            kind,
            observation_type,
            session_id,
            source,
            project,
            title,
            content,
            tags,
            facts,
            concepts,
            files_read,
            files_modified,
            narrative.unwrap_or(""),
            now,
            now
        ],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_vector(conn: &Connection, memory_id: i64, vector: &[f32]) -> Result<()> {
    let blob: &[u8] = bytemuck::cast_slice(vector);
    conn.execute(
        "INSERT OR REPLACE INTO vectors (memory_id, vector) VALUES (?1, ?2)",
        params![memory_id, blob],
    )?;
    Ok(())
}

pub fn get_memory(conn: &Connection, id: i64) -> Result<Memory> {
    conn.query_row(
        "SELECT id, kind, observation_type, session_id, source, project,
         title, content, tags, facts, concepts, files_read, files_modified, narrative,
         created_at_epoch, updated_at_epoch
         FROM memories WHERE id = ?1",
        params![id],
        |row| {
            Ok(Memory {
                id: row.get(0)?,
                kind: row.get(1)?,
                observation_type: row.get(2)?,
                session_id: row.get(3)?,
                source: row.get(4)?,
                project: row.get(5)?,
                title: row.get(6)?,
                content: row.get(7)?,
                tags: row.get(8)?,
                facts: row.get(9)?,
                concepts: row.get(10)?,
                files_read: row.get(11)?,
                files_modified: row.get(12)?,
                narrative: row.get(13)?,
                created_at_epoch: row.get(14)?,
                updated_at_epoch: row.get(15)?,
            })
        },
    )
    .map_err(|e| match e {
        rusqlite::Error::QueryReturnedNoRows => {
            MemAgentError::NotExist(format!("memory id={id}"))
        }
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
        "SELECT id, kind, observation_type, session_id, source, project,
         title, content, tags, facts, concepts, files_read, files_modified, narrative,
         created_at_epoch, updated_at_epoch
         FROM memories ORDER BY updated_at_epoch DESC LIMIT ?1",
    )?;

    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(Memory {
            id: row.get(0)?,
            kind: row.get(1)?,
            observation_type: row.get(2)?,
            session_id: row.get(3)?,
            source: row.get(4)?,
            project: row.get(5)?,
            title: row.get(6)?,
            content: row.get(7)?,
            tags: row.get(8)?,
            facts: row.get(9)?,
            concepts: row.get(10)?,
            files_read: row.get(11)?,
            files_modified: row.get(12)?,
            narrative: row.get(13)?,
            created_at_epoch: row.get(14)?,
            updated_at_epoch: row.get(15)?,
        })
    })?;

    let mut memories = Vec::new();
    for row in rows {
        memories.push(row?);
    }
    Ok(memories)
}

pub fn list_memories_by_kind(
    conn: &Connection,
    kind: &str,
    limit: usize,
) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, observation_type, session_id, source, project,
         title, content, tags, facts, concepts, files_read, files_modified, narrative,
         created_at_epoch, updated_at_epoch
         FROM memories WHERE kind = ?1
         ORDER BY updated_at_epoch DESC LIMIT ?2",
    )?;

    let rows = stmt.query_map(params![kind, limit as i64], |row| {
        Ok(Memory {
            id: row.get(0)?,
            kind: row.get(1)?,
            observation_type: row.get(2)?,
            session_id: row.get(3)?,
            source: row.get(4)?,
            project: row.get(5)?,
            title: row.get(6)?,
            content: row.get(7)?,
            tags: row.get(8)?,
            facts: row.get(9)?,
            concepts: row.get(10)?,
            files_read: row.get(11)?,
            files_modified: row.get(12)?,
            narrative: row.get(13)?,
            created_at_epoch: row.get(14)?,
            updated_at_epoch: row.get(15)?,
        })
    })?;

    let mut memories = Vec::new();
    for row in rows {
        memories.push(row?);
    }
    Ok(memories)
}

pub fn list_memories_by_session(
    conn: &Connection,
    session_id: &str,
) -> Result<Vec<Memory>> {
    let mut stmt = conn.prepare(
        "SELECT id, kind, observation_type, session_id, source, project,
         title, content, tags, facts, concepts, files_read, files_modified, narrative,
         created_at_epoch, updated_at_epoch
         FROM memories WHERE session_id = ?1
         ORDER BY created_at_epoch ASC",
    )?;

    let rows = stmt.query_map(params![session_id], |row| {
        Ok(Memory {
            id: row.get(0)?,
            kind: row.get(1)?,
            observation_type: row.get(2)?,
            session_id: row.get(3)?,
            source: row.get(4)?,
            project: row.get(5)?,
            title: row.get(6)?,
            content: row.get(7)?,
            tags: row.get(8)?,
            facts: row.get(9)?,
            concepts: row.get(10)?,
            files_read: row.get(11)?,
            files_modified: row.get(12)?,
            narrative: row.get(13)?,
            created_at_epoch: row.get(14)?,
            updated_at_epoch: row.get(15)?,
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
    title: Option<&str>,
    content: Option<&str>,
    tags: Option<&str>,
    observation_type: Option<&str>,
    narrative: Option<&str>,
) -> Result<()> {
    let mem = get_memory(conn, id)?;
    let new_title = title.unwrap_or(&mem.title);
    let new_content = content.unwrap_or(&mem.content);
    let new_tags = tags.unwrap_or(&mem.tags);
    let new_type = observation_type.unwrap_or(&mem.observation_type);
    let new_narrative = narrative.unwrap_or(mem.narrative.as_deref().unwrap_or(""));
    let now = get_epoch_now();

    let affected = conn.execute(
        "UPDATE memories SET title = ?1, content = ?2, tags = ?3, observation_type = ?4,
         narrative = ?5, updated_at_epoch = ?6 WHERE id = ?7",
        params![new_title, new_content, new_tags, new_type, new_narrative, now, id],
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
        .query_row("SELECT count(*) FROM memories", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|v| v as usize)?;

    let vector_count: usize = conn
        .query_row("SELECT count(*) FROM vectors", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|v| v as usize)?;

    let session_count: usize = conn
        .query_row("SELECT count(*) FROM sessions", [], |row| {
            row.get::<_, i64>(0)
        })
        .map(|v| v as usize)?;

    Ok(DbStats {
        memory_count,
        vector_count,
        session_count,
    })
}

pub fn get_stats_by_kind(conn: &Connection) -> Result<Vec<(String, usize)>> {
    let mut stmt =
        conn.prepare("SELECT kind, count(*) FROM memories GROUP BY kind ORDER BY count(*) DESC")?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as usize))
    })?;

    let mut stats = Vec::new();
    for row in rows {
        stats.push(row?);
    }
    Ok(stats)
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
        ensure_session(&conn, "sess-1", "/test", "opencode").unwrap();

        let id = insert_memory(
            &conn,
            "observation",
            "bugfix",
            Some("sess-1"),
            "opencode",
            "/test",
            "Fix login",
            "Fixed the auth middleware",
            "auto,tool,read,success",
            r#"["auth middleware was broken"]"#,
            r#"["gotcha"]"#,
            r#"[]"#,
            r#"["src/auth.ts"]"#,
            Some("Fixed the auth middleware by adding null check"),
        )
        .unwrap();

        assert!(id > 0);
        let mem = get_memory(&conn, id).unwrap();
        assert_eq!(mem.kind, "observation");
        assert_eq!(mem.observation_type, "bugfix");
        assert_eq!(mem.session_id, Some("sess-1".to_string()));
        assert_eq!(mem.title, "Fix login");
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
        let id = insert_memory(
            &conn, "observation", "change", None, "manual", "", "Test", "content",
            "", "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        let vec = vec![1.0f32, 2.0, 3.0, 4.0];

        insert_vector(&conn, id, &vec).unwrap();
        let (_, got) = get_memory_with_vector(&conn, id).unwrap();
        assert_eq!(got, Some(vec![1.0f32, 2.0, 3.0, 4.0]));
    }

    #[test]
    fn test_update_memory() {
        let conn = setup();
        let id = insert_memory(
            &conn, "observation", "change", None, "manual", "", "Old", "old", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        update_memory(&conn, id, Some("New"), None, None, Some("bugfix"), None).unwrap();
        let mem = get_memory(&conn, id).unwrap();
        assert_eq!(mem.title, "New");
        assert_eq!(mem.observation_type, "bugfix");
    }

    #[test]
    fn test_delete_memory() {
        let conn = setup();
        let id = insert_memory(
            &conn, "observation", "change", None, "manual", "", "Del", "bye",
            "", "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        assert!(delete_memory(&conn, id).unwrap());
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
        insert_memory(
            &conn, "observation", "change", None, "manual", "", "A", "a", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_memory(
            &conn, "observation", "change", None, "manual", "", "B", "b", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        let list = list_memories(&conn, 5).unwrap();
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn test_list_by_kind() {
        let conn = setup();
        insert_memory(
            &conn, "observation", "change", None, "manual", "", "Obs", "x", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_memory(
            &conn, "prompt", "change", None, "manual", "", "Prompt", "y", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        let obs = list_memories_by_kind(&conn, "observation", 5).unwrap();
        assert_eq!(obs.len(), 1);
        assert_eq!(obs[0].kind, "observation");
    }

    #[test]
    fn test_sessions() {
        let conn = setup();
        ensure_session(&conn, "sess-a", "/proj", "opencode").unwrap();
        let sess = get_session(&conn, "sess-a").unwrap();
        assert_eq!(sess.project, "/proj");
        assert_eq!(sess.source, "opencode");
    }

    #[test]
    fn test_load_all_vectors() {
        let conn = setup();
        let id1 = insert_memory(
            &conn, "observation", "change", None, "manual", "", "V1", "c1", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        let id2 = insert_memory(
            &conn, "observation", "change", None, "manual", "", "V2", "c2", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_vector(&conn, id1, &[1.0, 2.0]).unwrap();
        insert_vector(&conn, id2, &[3.0, 4.0]).unwrap();

        let entries = load_all_vectors(&conn).unwrap();
        assert_eq!(entries.len(), 2);
    }

    #[test]
    fn test_get_stats() {
        let conn = setup();
        let id = insert_memory(
            &conn, "observation", "change", None, "manual", "", "S", "t", "",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_vector(&conn, id, &[1.0]).unwrap();

        let stats = get_stats(&conn).unwrap();
        assert_eq!(stats.memory_count, 1);
        assert_eq!(stats.vector_count, 1);
    }
}
