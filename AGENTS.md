# mem-agent — Rust Persistent Memory Engine

## 🎯 Mission

Build a **high-performance persistent memory engine** for AI coding agents in Rust. Replace agentmemory's slow TypeScript brute-force kNN with a Rust-native hybrid search engine using **HNSW + SIMD + BM25 + ONNX embedding inference**.

Target: **<5ms hybrid search at 100K vectors**, zero external dependencies beyond SQLite.

---

## 📊 Current State

### What works now

```
mem-agent/src/
├── main.rs          # CLI (clap): add, search, list, get, update, delete, simulate, stats
├── db.rs            # SQLite + FTS5 schema + triggers
├── bm25.rs          # Custom BM25 implementation (pure Rust)
├── search.rs        # Search: FTS5 MATCH + BM25 re-rank + snippet generation
├── memory.rs        # CRUD + Memory/ SearchResult structs
└── simulation.rs    # 10 sample entries
```

### What's missing (from our research)

| Component | Missing | Target crate |
|-----------|---------|-------------|
| Vector embeddings | ❌ No ONNX inference | `candle` (17K⭐, pure Rust) |
| HNSW vector index | ❌ Brute-force only | `instant-distance` (346⭐) |
| SIMD cosine similarity | ❌ N/A (no vector search) | `wide` + `bytemuck` |
| Hybrid search (BM25 + Vector) | ❌ BM25-only | Custom `HybridSearch` |
| Incremental persistence | ❌ BM25 stats rebuilt per query | `rusqlite` + `bincode` |
| MCP server | ❌ CLI-only | `mcp-server` crate or stdio JSON |
| napi-rs bindings | ❌ No bridge needed (pure Rust) | — |

---

## 🧬 Architecture Target

```
┌──────────────────────────────────────────────────────────┐
│              mem-agent (pure Rust binary)                  │
│                                                            │
│  ┌─────────────────────────────────────────────────┐      │
│  │           HybridSearchEngine                      │      │
│  │                                                   │      │
│  │  ┌─────────────────┐  ┌─────────────────────┐   │      │
│  │  │  VectorIndex     │  │  TextIndex (BM25)    │   │      │
│  │  │  ┌───────────┐   │  │  ┌───────────────┐  │   │      │
│  │  │  │ HNSW      │   │  │  │ FTS5 (core)   │  │   │      │
│  │  │  │ instant-  │   │  │  │ tantivy or    │  │   │      │
│  │  │  │ distance  │   │  │  │ rusqlite FTS5 │  │   │      │
│  │  │  └───────────┘   │  │  └───────────────┘  │   │      │
│  │  │  ┌───────────┐   │  │  ┌───────────────┐  │   │      │
│  │  │  │ SIMD      │   │  │  │ Custom BM25   │  │   │      │
│  │  │  │ brute-    │   │  │  │ re-ranker     │  │   │      │
│  │  │  │ force     │   │  │  └───────────────┘  │   │      │
│  │  │  └───────────┘   │  └─────────────────────┘   │      │
│  │  └─────────────────┘                              │      │
│  │                                                   │      │
│  │  ┌─────────────────────────────────────────────┐  │      │
│  │  │   ONNX Embedding Engine (candle)             │  │      │
│  │  │   ┌────────────┐ ┌──────────┐ ┌──────────┐  │  │      │
│  │  │   │ BERT       │ │ Tokenizer│ │ Batch    │  │  │      │
│  │  │   │ Model      │ │ (Hugging │ │ Pipeline │  │  │      │
│  │  │   │ (ONNX)     │ │ Face)    │ │ (tokio)  │  │  │      │
│  │  │   └────────────┘ └──────────┘ └──────────┘  │  │      │
│  │  └─────────────────────────────────────────────┘  │      │
│  │                                                   │      │
│  │  ┌─────────────────────────────────────────────┐  │      │
│  │  │   Persistence Layer                           │  │      │
│  │  │   ┌──────────┐ ┌────────┐ ┌───────────────┐ │  │      │
│  │  │   │ SQLite   │ │bincode │ │ Memory-Mapped │ │  │      │
│  │  │   │ (rusqlite│ │serde   │ │ Vector Store  │ │  │      │
│  │  │   │ + FTS5)  │ │       │ │ (mmap)        │ │  │      │
│  │  │   └──────────┘ └────────┘ └───────────────┘ │  │      │
│  │  └─────────────────────────────────────────────┘  │      │
│  └─────────────────────────────────────────────────┘      │
│                                                            │
│  ┌──────────────┐  ┌──────────────┐  ┌────────────────┐   │
│  │ CLI (clap)   │  │ MCP Server   │  │ napi-rs addon  │   │
│  │ (hiện tại)   │  │ (stdio TCP)  │  │ (optional)     │   │
│  └──────────────┘  └──────────────┘  └────────────────┘   │
└──────────────────────────────────────────────────────────┘
```

---

## 📦 Dependencies cần thêm vào Cargo.toml

```toml
# === Vector Index ===
instant-distance = "0.7"          # HNSW ANN index
wide = "1"                        # SIMD f32x8/f32x16
bytemuck = "1"                    # Zero-cost cast &[f32] ↔ SIMD
rayon = "1"                       # Parallel scan

# === ONNX Embedding ===
candle-core = "0.9"               # Tensor ops
candle-onnx = "0.9"               # ONNX model loader
candle-transformers = "0.9"       # BERT model + tokenizer
tokenizers = "0.21"               # HuggingFace tokenizer (cần cho candle)
hf-hub = "0.4"                    # Download model từ HuggingFace Hub

# === Persistence ===
bincode = "2"                    # Binary serialization (thay JSON)
serde = { version = "1", features = ["derive"] }  # (đã có)
serde_json = "1.0"               # (đã có — giữ lại cho CLI output)

# === MCP Server (phase sau) ===
# mcp-server = "0.1"             # Khi thêm MCP
# tokio = { version = "1", features = ["full"] }  # Cho async MCP

# === Đã có ===
rusqlite = { version = "0.31", features = ["bundled-full", "fts5"] }
clap = { version = "4.5", features = ["derive"] }
chrono = "0.4"
unicode-segmentation = "1.11"
```

---

## 🗺️ Build Plan (5 Phases)

---

### Phase 1: SIMD Vector Index (nền tảng)

**Mục tiêu:** Thay thế brute-force bằng HNSW + SIMD exact kNN.

#### 1.1 Tạo `src/vector.rs` — Core Vector Types

```rust
//! Contiguous memory layout cho vectors
//! Dùng Vec<[f32; DIM]> thay vì HashMap<String, Float32Array>

pub const DEFAULT_DIM: usize = 768; // embeddinggemma-300m-ONNX

#[derive(Clone, Debug)]
pub struct VectorStore {
    pub ids: Vec<String>,
    pub vectors: Vec<Vec<f32>>,    // [N × DIM] contiguous
    pub metadata: Vec<VectorMeta>,
}

#[derive(Clone, Debug)]
pub struct VectorMeta {
    pub session_id: String,
    pub created_at: String,
}
```

**Key decisions:**
- `Vec<Vec<f32>>` là contiguous trong memory (mỗi vec là 1 allocation)
- Tối ưu sau: `Vec<[f32; 384]>` dùng `arrayvec` nếu DIM cố định
- IDs dùng `Vec<String>` riêng — search trả về index → lookup

#### 1.2 Tạo `src/distance.rs` — SIMD Cosine Similarity

```rust
//! SIMD-accelerated distance computation
//! Dùng `wide` crate cho portable SIMD (SSE/AVX/NEON)

fn cosine_simd(a: &[f32], b: &[f32]) -> f32 {
    // f32x16: 16 floats per operation
    // 384 dims → 24 iterations (384/16)
    // FMA: mul_add cho dot + norm đồng thời
}

fn cosine_scalar(a: &[f32], b: &[f32]) -> f32 {
    // Fallback scalar (cho remainder)
}
```

**Benchmark target:** 40-80× faster than TypeScript Float32Array loop.

#### 1.3 Tạo `src/hnsw.rs` — HNSW Index

```rust
//! HNSW (Hierarchical Navigable Small World) index
//! Dùng instant-distance crate

pub struct HnswIndex {
    index: instant_distance::HnswIndex,  // HNSW graph
    storage: instant_distance::Storage,  // Vector storage
    id_map: Vec<String>,                 // index → obsId
}
```

**API:**
```rust
impl HnswIndex {
    fn new(dims: usize) -> Self;
    fn insert(&mut self, id: &str, vector: &[f32]);
    fn search(&self, query: &[f32], k: usize) -> Vec<SearchHit>;
    fn exact_search(&self, query: &[f32], k: usize) -> Vec<SearchHit>; // SIMD fallback
    fn serialize(&self) -> Result<Vec<u8>>;
    fn deserialize(data: &[u8]) -> Result<Self>;
}
```

**Strategy:**
- HNSW cho 95% recall, O(log n) speed
- SIMD brute-force cho 100% exact khi cần (query thưa, index nhỏ)
- Auto-switch: nếu index < 1000 → exact, > 1000 → HNSW

#### 1.4 Benchmark Phase 1

```bash
cargo bench -- bench_vector_search
```

**Cần đo:**
- HNSW insert throughput (ops/s)
- HNSW search latency (p50, p99)
- SIMD brute-force latency
- HNSW recall @5, @10 (so với brute-force)
- Memory usage

---

### Phase 2: ONNX Embedding Engine

**Mục tiêu:** Chạy embedding model trong Rust, không cần Python/JS.

#### 2.1 Tạo `src/embed.rs` — Candle Embedding Engine

```rust
//! ONNX embedding inference với candle
//! Model: all-MiniLM-L6-v2 (384 dims, 22MB ONNX)

pub struct EmbeddingEngine {
    model: candle_onnx::Model,
    tokenizer: tokenizers::Tokenizer,
    device: candle_core::Device,  // CPU or CUDA
}

impl EmbeddingEngine {
    pub fn new(model_path: &str, use_gpu: bool) -> Result<Self>;
    
    /// Single vector embedding
    pub fn embed(&self, text: &str) -> Result<Vec<f32>>;
    
    /// Batched embedding (quan trọng: tối ưu throughput)
    pub fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;
}
```

**Download model:**
```bash
# embeddinggemma-300m-ONNX (~120MB, 768 dims)
git clone https://huggingface.co/onnx-community/embeddinggemma-300m-ONNX models/embeddinggemma-300m-ONNX
# Model files: models/embeddinggemma-300m-ONNX/onnx/model.onnx + model.onnx_data
# Tokenizer: models/embeddinggemma-300m-ONNX/tokenizer.json
```

#### 2.2 Batch Pipeline (cho index rebuild)

```rust
/// Batch processor: nhận texts, gửi batch lên model, trả về vectors
pub fn embed_corpus(
    engine: &EmbeddingEngine,
    texts: &[String],
    batch_size: usize,  // 32 default
) -> Result<Vec<Vec<f32>>> {
    // Chia thành batch 32 texts → 1 inference call
    // Amortize overhead ~4-8× speedup
}
```

#### 2.3 Benchmark Phase 2

- Single embed latency
- Batch embed throughput (batch 1, 8, 16, 32, 64)
- CPU vs GPU (Metal) comparison
- Memory usage

---

### Phase 3: Hybrid Search (BM25 + Vector)

**Mục tiêu:** Kết hợp FTS5 và vector search thành hybrid search engine.

#### 3.1 Tạo `src/hybrid.rs` — Hybrid Search Engine

```rust
//! Hybrid search: RRF fusion của BM25 + Vector + optional Graph

pub struct HybridSearch {
    fts: FtsIndex,          // SQLite FTS5
    vector: HnswIndex,      // HNSW vector index
    embed: EmbeddingEngine, // ONNX embedding
}

#[derive(Debug)]
pub struct HybridResult {
    pub memory: Memory,
    pub fts_score: f64,
    pub vector_score: f64,
    pub combined_score: f64,  // RRF fusion
    pub snippet: String,
}
```

**Search flow:**
```rust
impl HybridSearch {
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<HybridResult>> {
        // 1. Embed query → vector
        let qvec = self.embed.embed(query)?;
        
        // 2. BM25 search (FTS5)
        let fts_results = self.fts.search(query, limit * 3)?;
        
        // 3. Vector search (HNSW / SIMD)
        let vec_results = self.vector.search(&qvec, limit * 3)?;
        
        // 4. RRF fusion
        //    score = 0.4 · (1 / (60 + rank_fts)) + 0.6 · (1 / (60 + rank_vec))
        let fused = rrf_fuse(fts_results, vec_results, limit)?;
        
        // 5. Enrich: đọc full Memory từ SQLite
        // 6. Snippet generation
        // 7. Return top-K
        Ok(fused)
    }
}
```

**RRF fusion formula:**
```
score(d) = W_fts · (1 / (K + rank_fts(d))) + W_vec · (1 / (K + rank_vec(d)))
K = 60 (standard), W_fts = 0.4, W_vec = 0.6
```

#### 3.2 Sửa `src/search.rs` — Tích hợp hybrid

- Giữ nguyên FTS5 search (đã có)
- Thêm hybrid search function
- CLI flag: `--mode fts5|vector|hybrid`

#### 3.3 Sửa `src/main.rs` — CLI upgrade

```rust
enum Command {
    Add { title, content, tags },
    Search { query, mode, tags, limit },  // +mode: fts5|vector|hybrid
    Hybrid { query, limit },               // shorthand
    List { limit },
    Get { id },
    Update { id, title, content, tags },
    Delete { id },
    Index { action: IndexAction },          // build, rebuild, stats
    Simulate,
    Stats,
    Benchmark { phase: u8 },                // chạy benchmark
}

enum IndexAction {
    Build,     // Build HNSW + embed all
    Rebuild,   // Rebuild từ đầu
    Stats,     // Index statistics
}
```

#### 3.4 Benchmark Phase 3

- Hybrid search p50/p99 latency
- Recall @5, @10 (so với BM25-only)
- Throughput (QPS) vs index size

---

### Phase 4: Persistence & Index Lifecycle

**Mục tiêu:** Lưu HNSW index + vectors ra disk, rebuild khi cần.

#### 4.1 Tạo `src/persist.rs` — Index Persistence

```rust
//! Persistence cho HNSW + vectors + metadata
//! Binary format với bincode, SQLite cho metadata

pub struct IndexPersistence {
    db: Connection,              // SQLite (memory metadata + FTS5)
    hnsw_path: PathBuf,          // HNSW serialized file
    vector_store_path: PathBuf,  // Raw vectors (optional mmap)
}
```

**Strategy:**
- HNSW + vectors → `bincode` serialization (single file)
- Memory metadata (title, content, tags) → SQLite rows
- FTS5 → SQLite FTS5 table (đã có)
- Build index: iterate all memories → embed → insert HNSW → persist

#### 4.2 Index Rebuild Flow

```rust
pub fn rebuild_index(conn: &Connection) -> Result<()> {
    // 1. Load all memories from SQLite
    let memories = memory::load_all_for_index(conn)?;
    
    // 2. Embed all in batches (Phase 2 batch pipeline)
    //    32 texts per batch → 1 ONNX call
    let vectors = embed_corpus(&engine, &texts, 32)?;
    
    // 3. Build HNSW index
    let mut hnsw = HnswIndex::new(DEFAULT_DIM);
    for (i, (id, _)) in memories.iter().enumerate() {
        hnsw.insert(&id.to_string(), &vectors[i]);
    }
    
    // 4. Persist to disk
    let data = bincode::serialize(&hnsw)?;
    std::fs::write("index.hnsw.bin", data)?;
    
    // 5. Save vectors for mmap
    save_vectors_mmap("vectors.bin", &vectors)?;
}
```

**Key optimization:** `mmap` — load vectors trực tiếp từ disk mà không cần đọc vào RAM. OS quản lý page cache.

#### 4.3 CLI Commands

```bash
mem-agent index build      # Build full index từ SQLite
mem-agent index rebuild    # Rebuild từ đầu
mem-agent index stats      # Show index size, dims, entries
```

---

### Phase 5: MCP Server & Agent Integration

**Mục tiêu:** Cho phép AI agents (Claude Code, OpenCode, Cursor) dùng mem-agent qua MCP protocol.

#### 5.1 Tạo `src/mcp.rs` — MCP Server

```rust
//! Model Context Protocol server
//! STDIO transport (cho agent integration)

pub struct McpServer {
    hybrid: HybridSearch,
}

impl McpServer {
    pub fn run_stdio(&self);
    // Đọc JSON-RPC từ stdin, ghi response ra stdout
    // Tools:
    //   - memory_search(query, limit) → HybridResult[]
    //   - memory_add(title, content, tags) → id
    //   - memory_get(id) → Memory
    //   - memory_delete(id) → bool
    //   - index_stats() → stats
}
```

**MCP tools:**
```json
{
  "tools": [
    {
      "name": "memory_search",
      "description": "Hybrid search (BM25 + vector semantic)",
      "inputSchema": {
        "query": "string",
        "limit": "number (default: 10)",
        "mode": "'hybrid' | 'fts5' | 'vector'"
      }
    },
    {
      "name": "memory_add",
      "description": "Add a new memory entry"
    },
    {
      "name": "memory_get",
      "description": "Get memory by ID"
    }
  ]
}
```

#### 5.2 Agent Integration Config

**OpenCode (`opencode.json`):**
```json
{
  "mcp": {
    "mem-agent": {
      "type": "local",
      "command": ["/path/to/mem-agent", "mcp"],
      "enabled": true
    }
  }
}
```

**Claude Code (`~/.claude.json`):**
```json
{
  "mcpServers": {
    "mem-agent": {
      "command": "/path/to/mem-agent",
      "args": ["mcp"]
    }
  }
}
```

---

## 📈 Performance Targets

| Metric | Current (agentmemory TS) | Target (mem-agent Rust) | Speedup |
|--------|-------------------------|------------------------|---------|
| Vector search 100K | ~50ms (brute-force) | **~1ms (HNSW)** | **50×** |
| Exact kNN 100K | ~50ms | **~5ms (SIMD)** | **10×** |
| Embedding (single) | ~10ms (Xenova JS) | **~3ms (candle)** | **3×** |
| Embedding (batch 32) | ~150ms (sequential) | **~15ms (batched)** | **10×** |
| Hybrid search | ~14ms (FTS5+BM25+Vector) | **~3ms** | **4-5×** |
| Index persist flush | ~500ms (JSON 210MB) | **<10ms (bincode)** | **50×** |
| Memory usage (100K) | ~500MB (JS heap) | **~150MB (flat Vec)** | **3×** |
| Binary size | ~50MB (Node + Xenova) | **~8MB (static binary)** | — |

## Warm-start embedding (model loaded):
| Metric | Cold (first call) | Warm |
|--------|-------------------|------|
| Single embed | ~50ms (load model) | ~3ms |
| Batch 32 embed | ~200ms (load model) | ~15ms |

---

## 🧪 Test Strategy

### Unit tests (per module)

```bash
cargo test                    # All unit tests
cargo test -- --nocapture     # With stdout
```

### Benchmark tests

```bash
cargo bench                   # All benchmarks
cargo bench -- vector_search  # Vector-specific
cargo bench -- embed          # Embedding-specific
cargo bench -- hybrid         # Hybrid search
```

### Integration tests

```rust
#[test]
fn test_hybrid_search_accuracy() {
    // 1. Insert N test memories
    // 2. Build index
    // 3. Search known queries
    // 4. Assert recall @5 > 90%
}

#[test]
fn test_hnsw_recall_vs_bruteforce() {
    // So sánh HNSW vs SIMD brute-force trên cùng dataset
    // Assert recall > 95%
}
```

### Test dataset

Use `simulate` command với số lượng lớn:

```bash
mem-agent simulate --count 10000   # 10K sample memories
mem-agent index build               # Build HNSW + embed
mem-agent benchmark phase 1         # Run Phase 1 benchmarks
```

---

## 📁 File Structure Target

```
mem-agent/
├── Cargo.toml
├── AGENTS.md                # ← này
├── models/
│   ├── embeddinggemma-300m-ONNX/  # Embedding model (~120MB, 768d)
│   │   ├── onnx/
│   │   │   ├── model.onnx
│   │   │   └── model.onnx_data
│   │   └── tokenizer.json
├── src/
│   ├── main.rs             # CLI entry (đã có, mở rộng)
│   ├── db.rs               # SQLite + FTS5 (đã có)
│   ├── bm25.rs             # BM25 scoring (đã có)
│   ├── search.rs           # FTS5 + BM25 search (đã có, mở rộng)
│   ├── memory.rs           # Memory CRUD (đã có)
│   ├── simulation.rs       # Sample data (đã có)
│   │
│   ├── vector.rs           # [NEW] VectorStore + contiguous layout
│   ├── distance.rs         # [NEW] SIMD cosine similarity
│   ├── hnsw.rs             # [NEW] HNSW index (instant-distance)
│   ├── embed.rs            # [NEW] ONNX embedding (candle)
│   ├── hybrid.rs           # [NEW] Hybrid search (BM25 + Vector)
│   ├── persist.rs          # [NEW] Index persistence (bincode)
│   ├── mcp.rs              # [NEW] MCP server (phase 5)
│   └── bench.rs            # [NEW] Benches
└── memories.db             # SQLite database
```

---

## 🚫 Những gì KHÔNG cần làm

| Không làm | Lý do |
|-----------|-------|
| ❌ napi-rs bindings | mem-agent là pure Rust binary, không cần bridge qua Node.js |
| ❌ WebSocket transport cho MCP | STDIO đơn giản hơn, đủ cho local MCP |
| ❌ GPU CUDA | CPU inference đủ nhanh (3ms/embed), Metal là optional |
| ❌ Python binding | Giữ focus: Rust-native CLI + MCP |
| ❌ Distributed storage | Single-node SQLite đủ cho 100K+ memories |
| ❌ Web UI | CLI + MCP là đủ; viewer có thể sau |

---

## 🏁 Priority Order

```
Phase 1 ────► SIMD Vector Index (nền tảng cho mọi thứ)
  │
  ▼
Phase 2 ────► ONNX Embedding Engine (cần cho hybrid search)
  │
  ▼
Phase 3 ────► Hybrid Search (kết hợp BM25 + Vector)
  │
  ▼
Phase 4 ────► Persistence & Index Lifecycle (lưu index ra disk)
  │
  ▼
Phase 5 ────► MCP Server (cho agent integration)
```

Mỗi phase đều có benchmark riêng để verify speedup. **Không chuyển phase khi chưa đạt target.**

---

## 🎯 Definition of Done

Phase 1:
- [ ] `cargo bench` cho HNSW vs SIMD brute-force
- [ ] HNSW recall > 95% @10
- [ ] SIMD cosine > 40× faster than TS baseline
- [ ] Auto-switch HNSW/exact hoạt động

Phase 2:
- [ ] `cargo test` cho embed + embed_batch
- [ ] Model download tự động (hf-hub)
- [ ] Batch throughput: batch 32 = 4× lần single sequential

Phase 3:
- [ ] Hybrid search ra kết quả chính xác
- [ ] Score detailed trong output
- [ ] `mem-agent search --mode hybrid` hoạt động

Phase 4:
- [ ] `mem-agent index build` hoàn chỉnh
- [ ] Index persist/load không mất dữ liệu
- [ ] Incremental update: thêm memory → auto-update index

Phase 5:
- [ ] Claude Code / OpenCode kết nối được MCP
- [ ] `memory_search` tool trả kết quả hybrid
- [ ] `memory_add` + `memory_get` tools

---

*Plan generated from deep research of agentmemory (rohitg00/agentmemory) and Rust ecosystem. Last updated: June 2026.*
