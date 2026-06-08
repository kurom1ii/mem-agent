use rusqlite::{params, Connection};

use crate::core::error::Result;

pub fn escape_fts5_query(query: &str) -> String {
    let mut escaped = String::new();
    let trimmed = query.trim();

    if trimmed.is_empty() {
        return String::new();
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();

    for (i, token) in tokens.iter().enumerate() {
        if i > 0 {
            escaped.push(' ');
        }

        let sanitized = token.replace('"', "\"\"");

        escaped.push('"');
        escaped.push_str(&sanitized);
        escaped.push('"');
    }

    escaped
}

pub fn fts5_search_raw(conn: &Connection, query: &str, limit: usize) -> Result<Vec<(i64, f64)>> {
    let escaped = escape_fts5_query(query);

    if escaped.is_empty() {
        return Ok(Vec::new());
    }

    let sql = "SELECT rowid, bm25(memories_fts, 0.0, 10.0, 5.0) AS score
         FROM memories_fts
         WHERE memories_fts MATCH ?1
         ORDER BY score
         LIMIT ?2"
        .to_string();

    let mut stmt = conn.prepare(&sql)?;

    let rows = stmt.query_map(params![escaped, limit as i64], |row| {
        let id: i64 = row.get(0)?;
        let score: f64 = row.get(1)?;
        Ok((id, score))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }

    Ok(results)
}

pub fn fts5_rebuild(conn: &Connection) -> Result<()> {
    conn.execute(
        "INSERT INTO memories_fts(memories_fts) VALUES ('rebuild')",
        [],
    )?;
    Ok(())
}

pub fn fts5_stats(conn: &Connection) -> Result<(usize, usize)> {
    let num_docs: i64 = conn
        .query_row("SELECT count(*) FROM memories_fts", [], |row| row.get(0))
        .unwrap_or(0);

    Ok((num_docs as usize, 0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::ops::insert_memory;
    use crate::db::schema::init_db;
    use rusqlite::Connection;

    fn setup() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
        init_db(&conn).unwrap();

        insert_memory(
            &conn,
            "observation",
            "change",
            None,
            "manual",
            "",
            "Rust Programming",
            "Rust is a systems programming language",
            "rust",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_memory(
            &conn,
            "observation",
            "change",
            None,
            "manual",
            "",
            "Python Guide",
            "Python is great for data science",
            "python",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();
        insert_memory(
            &conn,
            "observation",
            "change",
            None,
            "manual",
            "",
            "Async Rust",
            "Rust async programming with tokio",
            "rust,async",
            "[]", "[]", "[]", "[]", None,
        )
        .unwrap();

        conn
    }

    #[test]
    fn test_escape_simple() {
        let result = escape_fts5_query("hello world");
        assert_eq!(result, "\"hello\" \"world\"");
    }

    #[test]
    fn test_escape_with_quotes() {
        let result = escape_fts5_query("say \"hello\" now");
        assert_eq!(result, "\"say\" \"\"\"hello\"\"\" \"now\"");
    }

    #[test]
    fn test_escape_empty() {
        assert_eq!(escape_fts5_query(""), "");
        assert_eq!(escape_fts5_query("   "), "");
    }

    #[test]
    fn test_escape_single_token() {
        assert_eq!(escape_fts5_query("rust"), "\"rust\"");
    }

    #[test]
    fn test_fts5_search_basic() {
        let conn = setup();
        let results = fts5_search_raw(&conn, "Rust", 10).unwrap();
        assert!(!results.is_empty());

        let ids: Vec<i64> = results.iter().map(|(id, _)| *id).collect();
        assert!(ids.len() >= 2);
    }

    #[test]
    fn test_fts5_search_no_match() {
        let conn = setup();
        let results = fts5_search_raw(&conn, "javazzzzz", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts5_search_empty_query() {
        let conn = setup();
        let results = fts5_search_raw(&conn, "", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_fts5_search_scores_order() {
        let conn = setup();
        let results = fts5_search_raw(&conn, "Rust programming", 5).unwrap();

        for i in 1..results.len() {
            assert!(results[i - 1].1 <= results[i].1);
        }
    }

    #[test]
    fn test_fts5_rebuild() {
        let conn = setup();
        fts5_rebuild(&conn).unwrap();

        let results = fts5_search_raw(&conn, "Rust", 10).unwrap();
        assert!(!results.is_empty());
    }
}
