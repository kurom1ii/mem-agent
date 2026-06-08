---
name: remember
description: Lưu một insight, quyết định, hoặc kiến thức vào mem-agent. Dùng khi người dùng nói "nhớ cái này", "lưu lại", "remember this".
argument-hint: "[nội dung cần nhớ]"
user-invocable: true
---

Người dùng muốn lưu nội dung này vào bộ nhớ dài hạn: $ARGUMENTS

Dùng MCP tool `memory_add` (từ mem-agent server được kết nối qua `.mcp.json`) để lưu.

Các bước:
1. Phân tích nội dung cần nhớ — rút ra insight, quyết định, hoặc facts chính.
2. Đặt `title` ngắn gọn (≤ 80 ký tự) mô tả nội dung.
3. Đặt `content` là nội dung đầy đủ cần nhớ.
4. Thêm `tags` là các từ khóa cách nhau bởi dấu phẩy (vd: "rust,async,performance").
5. Gọi `memory_add` với các fields:
   - `title` — tiêu đề
   - `content` — nội dung đầy đủ
   - `tags` — tags (có thể để trống "")
6. Xác nhận với người dùng rằng đã lưu thành công, kèm ID của memory.

Nếu `memory_add` không khả dụng:
1. Chạy `mem-agent download` để tải model ONNX vào `plugins/opencode/models/embeddinggemma-300m-ONNX`
2. Build `cargo build --release`
3. Kiểm tra `mem-agent` có trong PATH không
