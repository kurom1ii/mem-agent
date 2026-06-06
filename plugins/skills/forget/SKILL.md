---
name: forget
description: Xóa một memory khỏi mem-agent. Dùng khi người dùng nói "quên đi", "xóa memory", "forget this".
argument-hint: "[ID của memory cần xóa]"
user-invocable: true
---

Người dùng muốn xóa memory có ID: $ARGUMENTS

Dùng MCP tool `memory_delete` với `id` là ID cần xóa.

Các bước:
1. Parse ID từ $ARGUMENTS (có thể là số hoặc text chứa số).
2. Gọi `memory_delete` với `id` đã parse.
3. Nếu thành công: "Đã xóa memory [ID]"
4. Nếu không tìm thấy: "Memory [ID] không tồn tại"
5. Nếu cần tìm ID trước, dùng `recall` skill để search trước.
