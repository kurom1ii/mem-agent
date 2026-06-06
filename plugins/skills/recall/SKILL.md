---
name: recall
description: Tìm kiếm trong mem-agent các memory liên quan đến một chủ đề. Dùng khi người dùng nói "nhớ lại", "tìm kiếm", "recall", "what did we do".
argument-hint: "[từ khóa tìm kiếm]"
user-invocable: true
---

Người dùng muốn tìm kiếm memory về: $ARGUMENTS

Dùng MCP tool `memory_search` (từ mem-agent server) với query là nội dung tìm kiếm và `limit: 10`.

Các bước:
1. Gọi `memory_search` với:
   - `query` — từ khóa tìm kiếm (giữ nguyên ngôn ngữ của người dùng)
   - `limit` — 10 (mặc định)
   - `mode` — "fts5" (full-text search)
2. Hiển thị kết quả theo format:
   ```
   [ID:1] Tiêu đề (score: 0.85)
     Nội dung tóm tắt...
   ```
3. Nếu không có kết quả, gợi ý 2-3 từ khóa thay thế.
4. **Không tự bịa ra memory.** Chỉ hiển thị những gì MCP tool trả về.

Nếu `memory_search` không khả dụng, kiểm tra mem-agent MCP server đã chạy chưa.
