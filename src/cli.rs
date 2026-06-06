use crate::core::config::DEFAULT_LIMIT;
use crate::core::error::Result;
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
    },
    Search {
        query: String,
        #[arg(short, long, default_value = "fts5")]
        mode: String,
        #[arg(short, long)]
        tags: Option<String>,
        #[arg(short = 'n', long, default_value_t = DEFAULT_LIMIT as u64)]
        limit: u64,
    },
    List {
        #[arg(short = 'n', long, default_value_t = 20)]
        limit: usize,
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
        } => {
            let id = crate::db::ops::insert_memory(&conn, &title, &content, &tags)?;
            println!("✅ Memory added with ID: {id}");
        }
        Command::Search {
            query,
            mode,
            tags: _,
            limit,
        } => {
            let limit = limit as usize;
            let results = match mode.as_str() {
                "vector" | "hybrid" => crate::db::fts::fts5_search_raw(&conn, &query, limit)?,
                _ => crate::db::fts::fts5_search_raw(&conn, &query, limit)?,
            };

            if results.is_empty() {
                println!("No results found for: {query}");
            } else {
                println!("🔍 Found {} results for: {query}\n", results.len());
                for (i, (id, score)) in results.iter().enumerate() {
                    match crate::db::ops::get_memory(&conn, *id) {
                        Ok(mem) => {
                            println!(
                                "{}. [ID:{}] {} (score: {:.4})",
                                i + 1,
                                mem.id,
                                mem.title,
                                score
                            );
                            println!("   Tags: {}", mem.tags);
                            println!("   {}\n", &mem.content[..mem.content.len().min(150)]);
                        }
                        Err(_) => {
                            println!("{}. [ID:{}] (score: {:.4})\n", i + 1, id, score);
                        }
                    }
                }
            }
        }
        Command::List { limit } => {
            let mems = crate::db::ops::list_memories(&conn, limit)?;
            if mems.is_empty() {
                println!("No memories stored.");
            } else {
                println!("📋 Latest {} memories:\n", mems.len());
                for mem in &mems {
                    println!(
                        "[ID:{}] {} | tags: {} | updated: {}",
                        mem.id, mem.title, mem.tags, mem.updated_at
                    );
                }
            }
        }
        Command::Get { id } => match crate::db::ops::get_memory(&conn, id) {
            Ok(mem) => {
                println!("ID:       {}", mem.id);
                println!("Title:    {}", mem.title);
                println!("Content:  {}", mem.content);
                println!("Tags:     {}", mem.tags);
                println!("Created:  {}", mem.created_at);
                println!("Updated:  {}", mem.updated_at);
            }
            Err(e) => eprintln!("Error: {e}"),
        },
        Command::Update {
            id,
            title,
            content,
            tags,
        } => {
            let mem = crate::db::ops::get_memory(&conn, id)?;
            let new_title = title.as_deref().unwrap_or(&mem.title);
            let new_content = content.as_deref().unwrap_or(&mem.content);
            let new_tags = tags.as_deref().unwrap_or(&mem.tags);
            crate::db::ops::update_memory(&conn, id, new_title, new_content, new_tags)?;
            println!("✅ Memory {id} updated");
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
                println!("✅ Index built");
            }
            IndexAction::Rebuild => {
                crate::db::fts::fts5_rebuild(&conn)?;
                println!("✅ Index rebuilt");
            }
            IndexAction::Stats => {
                let stats = crate::db::ops::get_stats(&conn)?;
                let (docs, terms) = crate::db::fts::fts5_stats(&conn)?;
                println!("Memory count:  {}", stats.memory_count);
                println!("Vector count:  {}", stats.vector_count);
                println!("FTS docs:      {docs}");
                println!("FTS terms:     {terms}");
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
            println!("FTS docs:      {docs}");
            println!("FTS terms:     {terms}");
        }
        Command::Download => {
            let (model_path, tokenizer_path) = crate::download::ensure_model_downloaded()
                .map_err(crate::core::error::MemAgentError::Config)?;
            println!("Model: {model_path}");
            println!("Tokenizer: {tokenizer_path}");
        }
        Command::Verify => {
            println!("=== mem-agent ONNX Model Verification (ort) ===\n");

            let (_model_path, _tokenizer_path) =
                crate::download::ensure_model_downloaded()
                    .map_err(crate::core::error::MemAgentError::Config)?;

            println!("[1/2] Loading engine...");
            let engine = crate::embed::engine::EmbeddingEngine::from_pretrained()
                .map_err(crate::core::error::MemAgentError::Config)?;
            println!("  ✅ Engine loaded (dim: {})", engine.dim());

            println!("[2/2] Running inference...");
            let query = "Hello world from mem-agent";
            let start = std::time::Instant::now();
            let vec = engine.embed_query(query)
                .map_err(crate::core::error::MemAgentError::Config)?;
            let elapsed = start.elapsed();

            println!("  ✅ Inference: {:?}", elapsed);
            println!("\n=== Result ===");
            println!("  Query:        {query}");
            println!("  Embedding dim: {}", vec.len());
            println!("  First 5:      {:?}", &vec[..5]);
            println!("  Last 5:       {:?}", &vec[vec.len()-5..]);
            let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
            println!("  L2 norm:      {:.6}", norm);

            let doc = "Rust is a systems programming language";
            let start = std::time::Instant::now();
            let vec2 = engine.embed_document(doc)
                .map_err(crate::core::error::MemAgentError::Config)?;
            let elapsed2 = start.elapsed();

            println!("\n  Doc:          {doc}");
            println!("  Embedding dim: {}", vec2.len());
            println!("  Inference:    {:?}", elapsed2);

            let similarity: f32 = vec.iter().zip(vec2.iter()).map(|(a, b)| a * b).sum();
            println!("  Similarity:   {:.6}", similarity);

            println!("\n  ✅ Model hoạt động bình thường!");
        }
        Command::Mcp => {
            println!("Starting mem-agent MCP server...");
            let server = crate::mcp::server::McpServer::new(&cli.db)?;
            server.run_stdio();
        }
    }

    Ok(())
}
