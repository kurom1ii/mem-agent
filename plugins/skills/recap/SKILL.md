---
name: recap
description: Tóm tắt toàn bộ kiến thức đã lưu trong phiên hiện tại. Tự động gọi khi bắt đầu session mới. Dùng MCP tools để lấy thống kê và memory gần đây nhất.
user-invocable: true
---

Tự động recap kiến thức đã lưu khi bắt đầu session mới.

Dùng MCP tools:
- `index_stats` — lấy thống kê (tổng số memory, vectors)
- `memory_list` — lấy 20 memory gần nhất

Các bước:
1. Gọi `index_stats` để lấy thống kê.
2. Gọi `memory_list` với `limit: 20` để lấy memory gần đây.
3. Hiển thị summary:
   ```
   📊 mem-agent: 150 memories, 45 vectors
   📋 Gần đây nhất:
     [ID:150] Tiêu đề | tags: rust | 2026-01-01
     ...
   ```
4. Nếu có memory quan trọng (dựa vào tags), highlight cho người dùng.
