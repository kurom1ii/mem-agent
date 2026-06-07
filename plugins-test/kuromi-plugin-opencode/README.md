# kuromi-plugin-opencode

Demo plugin minh họa TOÀN BỘ hooks trong OpenCode.

## Cài đặt

```bash
cp -r plugins-test/kuromi-plugin-opencode ~/.config/opencode/plugins/kuromi-plugin-opencode
```

## Xem log

```bash
# Real-time
tail -f /home/kuromi/work/mywork/mem-agent/plugins-test/kuromi-plugin-opencode/log.log

# Xóa log cũ
rm /home/kuromi/work/mywork/mem-agent/plugins-test/kuromi-plugin-opencode/log.log
```

## Format log

```
[HH:MM:SS.mmm] COLOR_LABEL______________ >> key=value | key=value ...
```

## Danh sách hook được minh họa

### Session Lifecycle
- `🟢 session.created` — khi tạo phiên mới
- `⏳ session.status` — khi trạng thái phiên thay đổi
- `🗜️ session.compacted` — khi context bị nén
- `📝 session.updated` — khi metadata phiên cập nhật
- `📊 session.diff` — khi có diff giữa các lần chat
- `🔴 session.deleted` — khi phiên kết thúc
- `💥 session.error` — khi phiên gặp lỗi

### Message Events
- `💬 message.updated` — khi tin nhắn được cập nhật (cả user & assistant)
- `🗑️ message.removed` — khi tin nhắn bị xóa

### Message Part Events (chi tiết nhất)
- `🤖 part.subtask` — AI spawn subagent
- `🔧 tool.running` — tool bắt đầu chạy
- `✅ tool.completed` — tool chạy xong (có duration, input, output)
- `❌ tool.error` — tool gặp lỗi
- `🏁 part.step-finish` — AI kết thúc 1 bước reasoning
- `🧠 part.reasoning` — AI đang suy nghĩ
- `📄 part.file` — file được tham chiếu
- `🔀 part.patch` — code change được đề xuất
- `🗜️ part.compaction` — context bị nén
- `👤 part.agent` — AI chọn agent khác
- `🔄 part.retry` — AI thử lại

### Other Events
- `✏️ file.edited` — file bị chỉnh sửa
- `🔐 permission.updated` — OpenCode hỏi quyền
- `🔓 permission.replied` — user trả lời quyền
- `✅ todo.updated` — todo list thay đổi
- `⌨️ command.executed` — user chạy slash command

### Hook Handlers
- `📋 config` — load config
- `💬 chat.message` — user gửi/AI trả lời
- `⚙️ chat.params` — tham số chat thay đổi
- `🔄 system.transform` — sửa system prompt
- `⏩ tool.execute.before` — trước khi tool chạy
- `🗜️ session.compacting` — session compacting hook
