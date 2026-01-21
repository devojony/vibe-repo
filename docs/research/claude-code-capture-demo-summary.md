# Claude Code 消息捕获演示总结

## 演示成果

我已经创建了三个完整的演示程序，展示如何捕获 Claude Code 的消息历史：

### 📁 创建的文件

```
examples/
├── README.md                          # 完整的使用文档
├── capture-claude-messages.sh         # Bash 脚本演示
├── capture_claude_messages.py         # Python 演示
└── capture_claude_messages.rs         # Rust 演示
```

---

## 核心发现

### 1. Claude Code 的输出格式

Claude Code 使用 `--output-format=stream-json` 时，输出 **JSON 流**（每行一个 JSON 对象）：

```json
{"type":"session_id","session_id":"9f885e5f-b19e-4c31-9f26-4a783bf700b0"}
{"type":"system","subtype":"init","cwd":"/workspace",...}
{"type":"thinking","content":"我需要读取文件..."}
{"type":"tool_use","name":"Read","input":{...}}
{"type":"tool_result","content":"...","is_error":false}
{"type":"result","content":"任务完成"}
```

### 2. 消息类型

| 类型 | 说明 | 示例 |
|------|------|------|
| `session_id` | 会话ID | 用于恢复会话 |
| `system` | 系统消息 | 初始化信息、Hook 响应 |
| `thinking` | Agent 思考过程 | "我需要读取文件..." |
| `tool_use` | 工具调用 | Read, Edit, Bash 等 |
| `tool_result` | 工具执行结果 | 成功/失败 + 输出 |
| `result` | 最终结果 | Agent 的总结 |
| `error` | 错误信息 | 执行失败的错误 |
| `control_request` | 控制请求 | 权限批准请求 |

### 3. 关键启动参数

```bash
npx -y @anthropic-ai/claude-code@2.1.7 \
    --verbose \                          # 详细输出
    --output-format=stream-json \        # JSON 流式输出 ✅
    --include-partial-messages \         # 包含部分消息 ✅
    --permission-mode=bypassPermissions  # 自动批准权限 ✅
```

**为什么这些参数重要**：
- `stream-json`: 提供结构化的 JSON 输出，易于解析
- `include-partial-messages`: 实时获取 Agent 的思考过程
- `bypassPermissions`: 避免需要手动批准，适合自动化

---

## 实际运行结果

### Python 演示输出（部分）

```
=== Claude Code 消息捕获演示 (Python) ===

📁 工作目录: /tmp/claude-code-python-demo-1768800139

📝 创建了测试文件 test.py

💬 提示: 请为 test.py 文件添加一个函数注释

🚀 启动 Claude Code...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📨 原始消息: {"type":"session_id","session_id":"9f885e5f-b19e-4c31-9f26-4a783bf700b0"}
🔑 [会话ID] 9f885e5f-b19e-4c31-9f26-4a783bf700b0
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

📨 原始消息: {"type":"system","subtype":"init","cwd":"/private/tmp/claude-code-python-demo-1768800139",...}
📋 [其他] 类型: system
   数据: {
     "type": "system",
     "subtype": "init",
     "cwd": "/private/tmp/claude-code-python-demo-1768800139",
     "session_id": "9f885e5f-b19e-4c31-9f26-4a783bf700b0",
     "tools": ["Task", "Bash", "Read", "Edit", ...],
     "model": "claude-sonnet-4-5-20250929",
     "permissionMode": "bypassPermissions"
   }
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

**关键观察**：
1. ✅ 成功捕获了 JSON 流输出
2. ✅ 每条消息都有明确的 `type` 字段
3. ✅ 包含了完整的系统信息（工具列表、模型、权限模式等）
4. ✅ 可以实时解析和分类消息

---

## 在 VibeRepo 中的应用

### 消息捕获流程

```
┌─────────────────────────────────────────────────────────┐
│  1. 启动 Claude Code                                     │
│     npx @anthropic-ai/claude-code                       │
│     --output-format=stream-json                         │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│  2. 捕获 stdout (逐行)                                   │
│     while let Some(line) = stdout.next_line() {         │
│         process_line(line);                             │
│     }                                                    │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│  3. 解析 JSON                                            │
│     let msg: ClaudeMessage = serde_json::from_str(line)?│
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│  4. 规范化                                               │
│     match msg.type {                                    │
│         "thinking" => NormalizedEntry::Thinking {...}   │
│         "tool_use" => NormalizedEntry::ToolUse {...}    │
│         ...                                             │
│     }                                                    │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│  5. 存储到 MsgStore                                      │
│     msg_store.push(LogMsg::Stdout(line));               │
│     msg_store.push(LogMsg::JsonPatch(patch));           │
└────────────────────┬────────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│  6. 广播                                                 │
│     - 数据库持久化 (execution_logs 表)                   │
│     - WebSocket 推送 (实时前端更新)                      │
└─────────────────────────────────────────────────────────┘
```

### 代码映射

演示代码 → VibeRepo 实现：

| 演示代码 | VibeRepo 组件 | 说明 |
|---------|--------------|------|
| `subprocess.Popen()` | `docker.create_exec()` | 启动进程 |
| `for line in stdout` | `output.next().await` | 读取输出 |
| `json.loads(line)` | `serde_json::from_str()` | 解析 JSON |
| `process_message()` | `normalize_log_line()` | 规范化 |
| `messages.append()` | `msg_store.push()` | 存储消息 |

---

## 验证的设计假设

### ✅ 已验证

1. **JSON 流输出可用** - Claude Code 确实支持 `--output-format=stream-json`
2. **消息类型丰富** - 包含 thinking、tool_use、tool_result、result 等
3. **实时捕获可行** - 可以逐行读取并实时处理
4. **结构化数据** - JSON 格式易于解析和存储
5. **系统信息完整** - 包含 session_id、tools、model 等元数据

### 🔍 需要进一步验证

1. **大量输出的性能** - 长时间运行时的内存占用
2. **错误处理** - 各种异常情况的处理
3. **会话恢复** - 使用 session_id 恢复会话
4. **控制协议** - control_request 的处理和响应

---

## 下一步

### 1. 完善演示

- [ ] 添加会话恢复演示
- [ ] 添加控制请求处理演示
- [ ] 添加错误处理演示
- [ ] 添加性能测试

### 2. 集成到 VibeRepo

- [ ] 实现 ClaudeCodeExecutor
- [ ] 实现消息规范化
- [ ] 实现 MsgStore
- [ ] 实现 WebSocket 推送
- [ ] 编写集成测试

### 3. 扩展到其他 Agents

- [ ] Gemini CLI 消息捕获演示
- [ ] Codex 消息捕获演示
- [ ] 统一的 Executor 接口实现

---

## 运行演示

### Python 演示
```bash
cd examples
python3 capture_claude_messages.py
```

### Bash 演示
```bash
cd examples
./capture-claude-messages.sh
```

### Rust 演示
```bash
cd examples
# 需要先设置 Cargo 项目（见 README.md）
cargo run
```

---

## 参考资料

- [演示 README](./examples/README.md)
- [设计文档](./docs/plans/2026-01-19-message-history-tracking-design.md)
- [Claude Code 文档](https://docs.anthropic.com/claude-code)
