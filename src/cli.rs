use crate::core::config::DEFAULT_LIMIT;
use crate::core::error::{MemAgentError, Result};
use crate::search::smart::{
    format_search_results, list_recent_memories_smart, search_memories_smart, SearchMode,
    SearchScope,
};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "mem-agent",
    version,
    about = "High-performance persistent memory engine"
)]
pub struct Cli {
    #[arg(short, long, default_value = "memories.db")]
    pub db: String,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    Add {
        title: String,
        content: String,
        #[arg(short, long, default_value = "")]
        tags: String,
        #[arg(long, default_value = "manual")]
        kind: String,
        #[arg(long, default_value = "change")]
        obs_type: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "manual")]
        source: String,
        #[arg(long, default_value = "")]
        project: String,
    },
    Observe {
        #[arg(long)]
        tool_name: String,
        #[arg(long, default_value = "")]
        tool_input: String,
        #[arg(long)]
        tool_output: String,
        #[arg(long, default_value = ".")]
        cwd: String,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value = "opencode")]
        source: String,
        #[arg(long, default_value = "")]
        project: String,
        #[arg(long)]
        user_prompt: Option<String>,
        #[arg(long, default_value = "")]
        tags: String,
        #[arg(long)]
        write_to_db: bool,
    },
    Summarize {
        #[arg(long)]
        session_id: String,
        #[arg(long, default_value = "")]
        project: String,
        #[arg(long, default_value = "opencode")]
        source: String,
    },
    Search {
        query: String,
        #[arg(short, long, default_value = "fts5")]
        mode: String,
        #[arg(short = 's', long, default_value = "auto")]
        scope: String,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(short = 'n', long, default_value_t = DEFAULT_LIMIT as u64)]
        limit: u64,
    },
    List {
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
        #[arg(short = 's', long, default_value = "all")]
        scope: String,
        #[arg(long)]
        kind: Option<String>,
    },
    Get {
        id: i64,
    },
    Update {
        id: i64,
        #[arg(short, long)]
        title: Option<String>,
        #[arg(short, long)]
        content: Option<String>,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(long)]
        obs_type: Option<String>,
        #[arg(long)]
        narrative: Option<String>,
    },
    Delete {
        id: i64,
    },
    Index {
        #[command(subcommand)]
        action: IndexAction,
    },
    Simulate {
        #[arg(short, long, default_value_t = 10)]
        count: usize,
        #[arg(short = 'r', long)]
        random: bool,
    },
    Stats,
    Download,
    Verify,
    Mcp,
}

#[derive(Subcommand)]
pub enum IndexAction {
    Build,
    Rebuild,
    Stats,
}

pub fn run(cli: Cli) -> Result<()> {
    let conn = crate::db::schema::get_connection(&cli.db)?;

    match cli.command {
        Command::Add {
            title,
            content,
            tags,
            kind,
            obs_type,
            session_id,
            source,
            project,
        } => {
            if let Some(sid) = &session_id {
                crate::db::ops::ensure_session(&conn, sid, &project, &source)?;
            }
            let id = crate::db::ops::insert_memory(
                &conn, &kind, &obs_type, session_id.as_deref(), &source, &project,
                &title, &content, &tags,
                "[]", "[]", "[]", "[]", None,
            )?;
            let vector_status = best_effort_index_memory(&conn, id, &title, &content);
            println!("✅ Memory added with ID: {id} ({vector_status})");
        }
        Command::Observe {
            tool_name,
            tool_input,
            tool_output,
            cwd,
            session_id,
            source,
            project,
            user_prompt,
            tags,
            write_to_db,
        } => {
            let conn_result = if write_to_db {
                Some(crate::db::schema::get_connection(&cli.db)?)
            } else {
                None
            };

            if let Some(ref conn) = conn_result {
                let result = crate::observer::observe_and_store(
                    conn,
                    "observation",
                    session_id.as_deref(),
                    &source,
                    &project,
                    &tool_name,
                    &tool_input,
                    &tool_output,
                    &cwd,
                    &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                    user_prompt.as_deref(),
                    &tags,
                    &[],
                    &[],
                );
                match result {
                    Ok(id) => println!("✅ Observed and stored: memory ID {id}"),
                    Err(e) => eprintln!("❌ Observer error: {e}"),
                }
            } else {
                if let Some(client) = crate::observer::client::ObserverClient::from_env_or_config() {
                    match client.classify_observation(
                        &tool_name, &tool_input, &tool_output, &cwd,
                        &chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
                        user_prompt.as_deref(),
                    ) {
                        Some(output) => {
                            println!("Observation type: {}", output.observation_type);
                            println!("Title: {}", output.title);
                            println!("Facts: {:?}", output.facts);
                            println!("Concepts: {:?}", output.concepts);
                        }
                        None => eprintln!("Observer returned no result"),
                    }
                } else {
                    let output = crate::observer::classifier::classify_tool_fallback(
                        &tool_name, &tool_input, &tool_output,
                    );
                    println!("[fallback] Observation type: {}", output.observation_type);
                    println!("[fallback] Title: {}", output.title);
                }
            }
        }
        Command::Summarize {
            session_id,
            project,
            source,
        } => {
            match crate::observer::summarize_session(
                &conn, &session_id, &project, &source,
            ) {
                Ok(Some(id)) => println!("✅ Summary stored: memory ID {id}"),
                Ok(None) => println!("No memories in session to summarize"),
                Err(e) => eprintln!("❌ Summarize error: {e}"),
            }
        }
        Command::Search {
            query,
            mode,
            scope,
            tags: _,
            limit,
        } => {
            let limit = limit as usize;
            let response = search_memories_smart(
                &conn,
                &query,
                limit,
                SearchMode::parse(&mode),
                SearchScope::parse(&scope),
            )?;
            println!("{}", format_search_results(&response));
        }
        Command::List { limit, scope, kind } => {
            let mems = if let Some(k) = kind {
                let limit_fetch = limit.saturating_mul(4).max(limit);
                let all = crate::db::ops::list_memories_by_kind(&conn, &k, limit_fetch)?;
                all.into_iter().take(limit).collect()
            } else {
                list_recent_memories_smart(&conn, limit, SearchScope::parse(&scope))?
            };

            if mems.is_empty() {
                println!("No memories stored.");
            } else {
                println!("📋 Latest {} memories:\n", mems.len());
                for mem in &mems {
                    println!(
                        "[ID:{}] {} [kind={}] [type={}] | tags: {} | session: {}",
                        mem.id,
                        mem.title,
                        mem.kind,
                        mem.observation_type,
                        mem.tags,
                        mem.session_id.as_deref().unwrap_or("-"),
                    );
                }
            }
        }
        Command::Get { id } => match crate::db::ops::get_memory(&conn, id) {
            Ok(mem) => {
                println!("ID:          {}", mem.id);
                println!("Kind:        {}", mem.kind);
                println!("Obs Type:    {}", mem.observation_type);
                println!("Title:       {}", mem.title);
                println!("Content:     {}", mem.content);
                println!("Tags:        {}", mem.tags);
                println!("Session:     {}", mem.session_id.as_deref().unwrap_or("-"));
                println!("Source:      {}", mem.source);
                println!("Project:     {}", mem.project);
                if let Some(n) = &mem.narrative {
                    if !n.is_empty() {
                        println!("Narrative:   {n}");
                    }
                }
                println!("Facts:       {}", mem.facts);
                println!("Concepts:    {}", mem.concepts);
                println!("Files Read:  {}", mem.files_read);
                println!("Files Mod:   {}", mem.files_modified);
                println!(
                    "Created:     {}",
                    chrono::DateTime::from_timestamp(mem.created_at_epoch, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| mem.created_at_epoch.to_string())
                );
                println!(
                    "Updated:     {}",
                    chrono::DateTime::from_timestamp(mem.updated_at_epoch, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                        .unwrap_or_else(|| mem.updated_at_epoch.to_string())
                );
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        Command::Update {
            id,
            title,
            content,
            tags,
            obs_type,
            narrative,
        } => {
            crate::db::ops::update_memory(
                &conn, id,
                title.as_deref(),
                content.as_deref(),
                tags.as_deref(),
                obs_type.as_deref(),
                narrative.as_deref(),
            )?;
            let mem = crate::db::ops::get_memory(&conn, id)?;
            let vector_status = best_effort_index_memory(&conn, id, &mem.title, &mem.content);
            println!("✅ Memory {id} updated ({vector_status})");
        }
        Command::Delete { id } => {
            let deleted = crate::db::ops::delete_memory(&conn, id)?;
            if deleted {
                println!("✅ Memory {id} deleted");
            } else {
                println!("❌ Memory {id} not found");
            }
        }
        Command::Index { action } => match action {
            IndexAction::Build => {
                crate::db::fts::fts5_rebuild(&conn)?;
                let indexed = backfill_vectors(&conn, false)?;
                println!("✅ Index built (vectors indexed: {indexed})");
            }
            IndexAction::Rebuild => {
                crate::db::fts::fts5_rebuild(&conn)?;
                let indexed = backfill_vectors(&conn, true)?;
                println!("✅ Index rebuilt (vectors indexed: {indexed})");
            }
            IndexAction::Stats => {
                let stats = crate::db::ops::get_stats(&conn)?;
                let (docs, terms) = crate::db::fts::fts5_stats(&conn)?;
                println!("Memory count:  {}", stats.memory_count);
                println!("Vector count:  {}", stats.vector_count);
                println!("Session count: {}", stats.session_count);
                println!("FTS docs:      {docs}");
                println!("FTS terms:     {terms}");

                if let Ok(by_kind) = crate::db::ops::get_stats_by_kind(&conn) {
                    println!("\nBy kind:");
                    for (k, c) in &by_kind {
                        println!("  {k}: {c}");
                    }
                }
            }
        },
        Command::Simulate { count, random } => {
            if random {
                crate::simulation::generate_random_samples(&conn, count)?;
            } else {
                crate::simulation::generate_samples(&conn, count)?;
            }
            println!("✅ Generated {count} sample memories");
        }
        Command::Stats => {
            let stats = crate::db::ops::get_stats(&conn)?;
            let (docs, terms) = crate::db::fts::fts5_stats(&conn)?;
            println!("Memory count:  {}", stats.memory_count);
            println!("Vector count:  {}", stats.vector_count);
            println!("Session count: {}", stats.session_count);
            println!("FTS docs:      {docs}");
            println!("FTS terms:     {terms}");

            if let Ok(by_kind) = crate::db::ops::get_stats_by_kind(&conn) {
                println!("\nBy kind:");
                for (k, c) in &by_kind {
                    println!("  {k}: {c}");
                }
            }
        }
        Command::Download => {
            let (model_path, tokenizer_path) = crate::download::ensure_model_downloaded()
                .map_err(crate::core::error::MemAgentError::Config)?;
            println!("Model: {model_path}");
            println!("Tokenizer: {tokenizer_path}");
        }
        Command::Verify => {
            println!("=== mem-agent ONNX Model Verification (ort) ===\n");

            let (_model_path, _tokenizer_path) = crate::download::ensure_model_downloaded()
                .map_err(crate::core::error::MemAgentError::Config)?;

            println!("[1/2] Loading engine...");
            let engine = crate::embed::engine::EmbeddingEngine::from_pretrained()
                .map_err(crate::core::error::MemAgentError::Config)?;
            println!("  ✅ Engine loaded (dim: {}, GPU: enabled)", engine.dim());

            println!("[2/2] Running inference...");
            let query = "Rust have cargo to install packages";
            let start = std::time::Instant::now();
            let vec = engine
                .embed_query(query)
                .map_err(crate::core::error::MemAgentError::Config)?;
            let elapsed = start.elapsed();

            println!("  ✅ Inference: {elapsed:?}");
            println!("\n=== Result ===");
            println!("  Query:        {query}");
            println!("  Embedding dim: {}", vec.len());
            println!("  First 5:      {:?}", &vec[..5]);
            println!("  Last 5:       {:?}", &vec[vec.len() - 5..]);
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("  L2 norm:      {:.6}", norm);

            let doc = "Rust is a systems programming language";
            let start = std::time::Instant::now();
            let vec2 = engine
                .embed_document(doc)
                .map_err(crate::core::error::MemAgentError::Config)?;
            let elapsed2 = start.elapsed();

            println!("\n  Doc:          {doc}");
            println!("  Embedding dim: {}", vec2.len());
            println!("  Inference:    {elapsed2:?}");

            let similarity: f32 = vec.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
            println!("  Similarity:   {:.6}", similarity);

            println!("\n  ✅ Model hoạt động bình thường!");
        }
        Command::Mcp => {
            let server = crate::mcp::server::McpServer::new(&cli.db)?;
            server.run_stdio();
        }
    }

    crate::db::schema::close_and_cleanup(conn, &cli.db).ok();
    Ok(())
}

fn best_effort_index_memory(
    conn: &rusqlite::Connection,
    id: i64,
    title: &str,
    content: &str,
) -> String {
    let combined = format!("title: {title}\ncontent: {content}");
    match load_embedding_engine_for_cli().and_then(|engine| {
        engine
            .embed_document(&combined)
            .map_err(MemAgentError::Embed)
    }) {
        Ok(vector) => match crate::db::ops::insert_vector(conn, id, &vector) {
            Ok(()) => "vector indexed".to_string(),
            Err(e) => format!("memory stored, vector insert failed: {e}"),
        },
        Err(e) => format!("memory stored, vector skipped: {e}"),
    }
}

fn load_embedding_engine_for_cli() -> Result<crate::embed::engine::EmbeddingEngine> {
    let (model_path, tokenizer_path) =
        crate::download::ensure_model_downloaded().map_err(MemAgentError::Config)?;
    let tokenizer = crate::embed::tokenizer_embed::TokenizerWrapper::from_file(&tokenizer_path)
        .map_err(MemAgentError::Config)?;

    crate::embed::engine::EmbeddingEngine::new(&model_path, tokenizer.clone())
        .init()
        .or_else(|gpu_err| {
            crate::embed::engine::EmbeddingEngine::new(&model_path, tokenizer)
                .cpu_only()
                .init()
                .map_err(|cpu_err| {
                    MemAgentError::Config(format!(
                        "GPU init failed: {gpu_err}; CPU fallback failed: {cpu_err}"
                    ))
                })
        })
}

fn backfill_vectors(conn: &rusqlite::Connection, clear_existing: bool) -> Result<usize> {
    if clear_existing {
        conn.execute("DELETE FROM vectors", [])?;
    }

    let engine = load_embedding_engine_for_cli()?;
    let mut stmt = conn.prepare("SELECT id, title, content FROM memories ORDER BY id ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;

    let mut indexed = 0usize;
    for row in rows {
        let (id, title, content) = row?;
        let combined = format!("title: {title}\ncontent: {content}");
        let vector = engine
            .embed_document(&combined)
            .map_err(MemAgentError::Embed)?;
        crate::db::ops::insert_vector(conn, id, &vector)?;
        indexed += 1;
    }

    Ok(indexed)
}
