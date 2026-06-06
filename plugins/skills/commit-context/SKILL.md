---
name: commit-context
description: Lấy context memory liên quan đến các file trong commit gần đây. Dùng trước khi commit để xem lịch sử liên quan.
argument-hint: "[commit message hoặc file paths]"
user-invocable: true
---

Trước khi commit, kiểm tra memory context liên quan đến các file sẽ commit: $ARGUMENTS

Dùng MCP tools:
- `memory_search` — tìm memory liên quan đến tên file hoặc nội dung commit message
- `memory_list` — xem memory gần đây nhất

Các bước:
1. Parse file paths từ $ARGUMENTS (hoặc từ git diff --name-only).
2. Với mỗi file, gọi `memory_search` với query là tên file.
3. Tổng hợp kết quả thành context trước commit:
   ```
   ## Commit Context (mem-agent)
   ### Files in this commit
   - src/main.rs — past edits: [ID:42] "Refactored error handling"
   ```
4. Nếu không có memory liên quan, chỉ báo "No related memories found".
