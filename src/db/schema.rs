use rusqlite::{params, Connection};

use crate::core::error::Result;

const SCHEMA_VERSION: i32 = 2;

pub fn get_connection(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    if db_path != ":memory:" {
        let _ = conn.pragma_update(None, "journal_mode", "WAL");
    }
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn checkpoint(conn: &Connection) -> Result<()> {
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
    Ok(())
}

pub fn close_and_cleanup(conn: Connection, db_path: &str) -> Result<()> {
    let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode=DELETE; PRAGMA wal_checkpoint(TRUNCATE);");
    drop(conn);
    let shm = format!("{db_path}-shm");
    let wal = format!("{db_path}-wal");
    let _ = std::fs::remove_file(&shm);
    let _ = std::fs::remove_file(&wal);
    Ok(())
}

pub fn init_db(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT 'opencode',
            title TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'active'
                CHECK(status IN ('active','completed','failed')),
            created_at_epoch INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at_epoch INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL DEFAULT 'manual'
                CHECK(kind IN ('observation','summary','prompt','manual')),
            observation_type TEXT NOT NULL DEFAULT 'change',
            session_id TEXT
                REFERENCES sessions(id) ON DELETE SET NULL,
            source TEXT NOT NULL DEFAULT 'manual',
            project TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '',
            facts TEXT NOT NULL DEFAULT '[]',
            concepts TEXT NOT NULL DEFAULT '[]',
            files_read TEXT NOT NULL DEFAULT '[]',
            files_modified TEXT NOT NULL DEFAULT '[]',
            narrative TEXT DEFAULT '',
            created_at_epoch INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at_epoch INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS vectors (
            memory_id INTEGER PRIMARY KEY
                REFERENCES memories(id) ON DELETE CASCADE,
            vector BLOB NOT NULL
        );",
    )?;

    create_indexes(conn)?;
    create_fts_table(conn)?;
    create_triggers(conn)
}

fn create_indexes(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS idx_memories_kind ON memories(kind);
         CREATE INDEX IF NOT EXISTS idx_memories_type ON memories(observation_type);
         CREATE INDEX IF NOT EXISTS idx_memories_session ON memories(session_id);
         CREATE INDEX IF NOT EXISTS idx_memories_source ON memories(source);
         CREATE INDEX IF NOT EXISTS idx_memories_project_time ON memories(project, created_at_epoch DESC);
         CREATE INDEX IF NOT EXISTS idx_memories_kind_type ON memories(kind, observation_type);
         CREATE INDEX IF NOT EXISTS idx_sessions_project ON sessions(project);
         CREATE INDEX IF NOT EXISTS idx_sessions_source ON sessions(source);
         CREATE INDEX IF NOT EXISTS idx_sessions_created ON sessions(created_at_epoch DESC);",
    )?;
    Ok(())
}

fn create_fts_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS memories_fts
        USING fts5(
            title,
            content,
            tags,
            narrative,
            facts,
            concepts,
            content='memories',
            content_rowid='id',
            tokenize='porter unicode61'
        );",
    )?;
    Ok(())
}

fn create_triggers(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TRIGGER IF NOT EXISTS memories_ai
        AFTER INSERT ON memories
        BEGIN
            INSERT INTO memories_fts(rowid, title, content, tags, narrative, facts, concepts)
            VALUES (new.id, new.title, new.content, new.tags, new.narrative, new.facts, new.concepts);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_ad
        AFTER DELETE ON memories
        BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, tags, narrative, facts, concepts)
            VALUES ('delete', old.id, old.title, old.content, old.tags, old.narrative, old.facts, old.concepts);
        END;

        CREATE TRIGGER IF NOT EXISTS memories_au
        AFTER UPDATE ON memories
        BEGIN
            INSERT INTO memories_fts(memories_fts, rowid, title, content, tags, narrative, facts, concepts)
            VALUES ('delete', old.id, old.title, old.content, old.tags, old.narrative, old.facts, old.concepts);
            INSERT INTO memories_fts(rowid, title, content, tags, narrative, facts, concepts)
            VALUES (new.id, new.title, new.content, new.tags, new.narrative, new.facts, new.concepts);
        END;",
    )?;
    Ok(())
}

fn has_column(conn: &Connection, table: &str, column: &str) -> bool {
    let sql = format!("SELECT count(*) FROM pragma_table_info('{table}') WHERE name = '{column}'");
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count > 0)
        .unwrap_or(false)
}

fn get_schema_version(conn: &Connection) -> Result<i32> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER NOT NULL DEFAULT 0)",
        [],
    )
    .ok();

    let v: i32 = conn
        .query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_version",
            [],
            |row| row.get(0),
        )
        .unwrap_or(0);
    Ok(v)
}

fn set_schema_version(conn: &Connection, version: i32) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO schema_version (version) VALUES (?1)",
        params![version],
    )?;
    Ok(())
}

pub fn run_migrations(conn: &Connection) -> Result<()> {
    let version = get_schema_version(conn)?;

    if version < 1 {
        migrate_to_v1(conn)?;
    }
    if version < 2 {
        migrate_to_v2(conn)?;
    }

    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, definition: &str) -> Result<()> {
    if has_column(conn, table, column) {
        return Ok(());
    }
    let sql = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    conn.execute(&sql, [])?;
    Ok(())
}

fn table_exists(conn: &Connection, table: &str) -> bool {
    let sql = format!("SELECT count(*) FROM sqlite_master WHERE type='table' AND name='{table}'");
    conn.query_row(&sql, [], |row| row.get::<_, i64>(0))
        .map(|count| count > 0)
        .unwrap_or(false)
}

fn migrate_to_v1(conn: &Connection) -> Result<()> {
    if !table_exists(conn, "memories") {
        set_schema_version(conn, 1)?;
        return Ok(());
    }

    ensure_column(conn, "memories", "kind", "TEXT NOT NULL DEFAULT 'manual'")?;
    ensure_column(conn, "memories", "observation_type", "TEXT NOT NULL DEFAULT 'change'")?;
    ensure_column(conn, "memories", "session_id", "TEXT")?;
    ensure_column(conn, "memories", "source", "TEXT NOT NULL DEFAULT 'manual'")?;
    ensure_column(conn, "memories", "project", "TEXT NOT NULL DEFAULT ''")?;
    ensure_column(conn, "memories", "facts", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(conn, "memories", "concepts", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(conn, "memories", "files_read", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(conn, "memories", "files_modified", "TEXT NOT NULL DEFAULT '[]'")?;
    ensure_column(conn, "memories", "narrative", "TEXT DEFAULT ''")?;
    ensure_column(conn, "memories", "created_at_epoch", "INTEGER NOT NULL DEFAULT 0")?;
    ensure_column(conn, "memories", "updated_at_epoch", "INTEGER NOT NULL DEFAULT 0")?;

    if has_column(conn, "memories", "created_at") && has_column(conn, "memories", "created_at_epoch") {
        conn.execute_batch(
            "UPDATE memories SET
                created_at_epoch = COALESCE(unixepoch(created_at), 0),
                updated_at_epoch = COALESCE(unixepoch(updated_at), 0)
             WHERE created_at_epoch = 0;",
        )?;
    }

    set_schema_version(conn, 1)?;
    Ok(())
}

fn migrate_to_v2(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY,
            project TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT 'opencode',
            title TEXT DEFAULT '',
            status TEXT NOT NULL DEFAULT 'active'
                CHECK(status IN ('active','completed','failed')),
            created_at_epoch INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at_epoch INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            kind TEXT NOT NULL DEFAULT 'manual'
                CHECK(kind IN ('observation','summary','prompt','manual')),
            observation_type TEXT NOT NULL DEFAULT 'change',
            session_id TEXT
                REFERENCES sessions(id) ON DELETE SET NULL,
            source TEXT NOT NULL DEFAULT 'manual',
            project TEXT NOT NULL DEFAULT '',
            title TEXT NOT NULL,
            content TEXT NOT NULL,
            tags TEXT DEFAULT '',
            facts TEXT NOT NULL DEFAULT '[]',
            concepts TEXT NOT NULL DEFAULT '[]',
            files_read TEXT NOT NULL DEFAULT '[]',
            files_modified TEXT NOT NULL DEFAULT '[]',
            narrative TEXT DEFAULT '',
            created_at_epoch INTEGER NOT NULL DEFAULT (unixepoch()),
            updated_at_epoch INTEGER NOT NULL DEFAULT (unixepoch())
        );

        CREATE TABLE IF NOT EXISTS vectors (
            memory_id INTEGER PRIMARY KEY
                REFERENCES memories(id) ON DELETE CASCADE,
            vector BLOB NOT NULL
        );",
    )?;

    create_indexes(conn)?;

    conn.execute_batch(
        "DROP TRIGGER IF EXISTS memories_ai;
         DROP TRIGGER IF EXISTS memories_ad;
         DROP TRIGGER IF EXISTS memories_au;
         DROP TABLE IF EXISTS memories_fts;",
    )?;
    create_fts_table(conn)?;
    create_triggers(conn)?;

    set_schema_version(conn, 2)?;
    Ok(())
}
