use rusqlite::Connection;

use crate::core::error::Result;

pub fn get_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    if db_path != ":memory:" {
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
    }
    init_db(&conn)?;
    Ok(conn)
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '',
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS vectors (
            memory_id INTEGER PRIMARY KEY
                REFERENCES memories(id) ON DELETE CASCADE,
            vector BLOB NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        );",
    )?;

    create_fts_table(conn)?;
    create_triggers(conn)
}

fn create_fts_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts
        USING fts5(
            title,
            content,
            tags,
            content='memories',
            content_rowid='id'
        );",
    )?;
    Ok(())
}

fn create_triggers(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS memories_ai
        AFTER INSERT ON memories
        BEGIN
            INSERT INTO memories_fts(rowid, title, content, tags)
            VALUES (new.id, new.title, new.content, new.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_ad
        AFTER DELETE ON memories
        BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, tags)
            VALUES ('delete', old.id, old.title, old.content, old.tags);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_au
        AFTER UPDATE ON memories
        BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, tags)
            VALUES ('delete', old.id, old.title, old.content, old.tags);
            INSERT INTO memories_fts(rowid, title, content, tags)
            VALUES (new.id, new.title, new.content, new.tags);
        END;",
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn test_init_db() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();

        conn.execute(
            "INSERT INTO memories (title, content, tags) VALUES (?1, ?2, ?3)",
            rusqlite::params!["Test", "Content here", "rust,test"],
        )
        .unwrap();

        let row: (i64, String, String) = conn
            .query_row(
                "SELECT id, title, content FROM memories WHERE title = 'Test'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(row.1, "Test");
        assert_eq!(row.2, "Content here");

        let fts_row: (i64,) = conn
            .query_row(
                "SELECT rowid FROM memories_fts WHERE memories_fts MATCH 'Content'",
                [],
                |row| Ok((row.get(0)?,)),
            )
            .unwrap();
        assert_eq!(fts_row.0, row.0);

        conn.execute("DELETE FROM memories WHERE id = ?1", rusqlite::params![row.0])
            .unwrap();

        let fts_count: i64 = conn
            .query_row(
                "SELECT count(*) FROM memories_fts WHERE memories_fts MATCH 'Content'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(fts_count, 0);
    }
}
