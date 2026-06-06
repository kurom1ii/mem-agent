---
name: session-history
description: Xem danh sách memory gần đây. Dùng khi người dùng nói "xem lịch sử", "những gì đã lưu", "session history".
user-invocable: true
---

Người dùng muốn xem danh sách memory gần đây.

Dùng MCP tool `memory_list` với `limit: 20`.

Các bước:
1. Gọi `memory_list` với `limit: 20`.
2. Hiển thị kết quả dạng bảng:
   ```
   [ID:1] Tiêu đề | tags: rust,async | updated: 2026-01-01
   [ID:2] Tiêu đề khác | tags: python | updated: 2026-01-02
   ```
3. Nếu không có memory nào: "Chưa có memory nào được lưu."
