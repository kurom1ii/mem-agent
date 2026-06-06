---
name: handoff
description: Chuẩn bị context để handoff sang session khác. Tổng hợp các memory quan trọng nhất để inject vào session mới.
user-invocable: true
---

Chuẩn bị context handoff cho session mới.

Dùng MCP tools:
- `memory_list` — lấy memory gần đây
- `memory_search` — tìm memory quan trọng

Các bước:
1. Gọi `memory_search` với query là "important decision architecture pattern" và `limit: 10`.
2. Gọi `memory_list` với `limit: 10`.
3. Tổng hợp thành một context block để inject vào session mới.
4. Format:
   ```
   ## Memory Context (mem-agent)
   ### Key Decisions
   - [ID:1] Decision A
   ### Recent Activity
   - [ID:150] Recent change B
   ```
