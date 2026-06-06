---
name: commit-history
description: Xem lịch sử memory liên quan đến các commit gần đây. Dùng sau khi commit để lưu context hoặc xem patterns.
user-invocable: true
---

Xem lịch sử memory gắn với các commit gần đây.

Dùng MCP tools:
- `memory_list` — lấy 30 memory gần nhất
- `memory_search` — tìm memory liên quan đến commit patterns

Các bước:
1. Gọi `memory_list` với `limit: 30`.
2. Lọc các memory có tags liên quan đến commit (git, commit, refactor, fix, feat).
3. Gọi `memory_search` với query "commit pattern recent change" để tìm patterns.
4. Hiển thị:
   ```
   ## Recent Commit-related Memories
   - [ID:50] feat: Added GPU support | tags: rust,gpu,cuda,commit
   - [ID:49] fix: Memory leak in vector store | tags: rust,fix,bug,commit
   ```
5. Đề xuất: nếu phát hiện patterns (vd: nhiều fix liên tiếp), suggest `memory_add` để lưu bài học.
