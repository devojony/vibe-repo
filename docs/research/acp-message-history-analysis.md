# ACP 协议消息历史获取能力分析

## 研究日期
2026-01-19

## 核心问题
**ACP 可以获取到 message history 吗？**

---

## 答案：✅ 可以

ACP 协议**完全支持**获取消息历史，并且提供了多种方式来访问和流式传输消息。

---

## ACP 的消息历史功能

### 1. 会话管理 (Session Management)

**API 端点**: `GET /session/{session_id}`

**响应内容**:
```json
{
  "id": "session-uuid",
  "history": [
    "uri://message/1",
    "uri://message/2",
    "uri://message/3"
  ],
  "state": "uri://state/current"
}
```

**说明**:
- ✅ 每个会话维护完整的消息历史
- ✅ 历史记录以 URI 数组形式存储
- ✅ 支持跨多次交互的状态保持

### 2. 事件流 (Event Streaming)

**API 端点**: `GET /runs/{run_id}/events`

**支持的事件类型**:
```
消息事件:
- message.created      # 消息创建
- message.part         # 消息部分（流式传输）
- message.completed    # 消息完成

运行事件:
- run.created          # 运行创建
- run.in-progress      # 运行进行中
- run.awaiting         # 运行等待
- run.completed        # 运行完成
- run.failed           # 运行失败
- run.cancelled        # 运行取消

错误事件:
- error                # 错误信息
```

**响应格式**:
```json
{
  "events": [
    {
      "type": "message.created",
      "timestamp": "2026-01-19T12:00:00Z",
      "data": {
        "message_id": "msg-123",
        "content": "..."
      }
    },
    {
      "type": "message.part",
      "timestamp": "2026-01-19T12:00:01Z",
      "data": {
        "message_id": "msg-123",
        "delta": "partial content..."
      }
    }
  ]
}
```

### 3. 实时流式传输 (Real-Time Streaming)

**特性**:
- ✅ 支持流式响应（Streaming Responses）
- ✅ 实时反馈 Agent 的执行过程
- ✅ 增量更新（Delta Updates）

**传输方式**:
- **REST API**: 通过 HTTP 长连接
- **Server-Sent Events (SSE)**: 实时推送事件
- **WebSocket**: 双向实时通信（可选）

### 4. 消息结构

**消息包含**:
```json
{
  "id": "msg-123",
  "parts": [
    {
      "content_type": "text/plain",
      "content": "消息内容",
      "metadata": {
        "timestamp": "2026-01-19T12:00:00Z",
        "source": "agent"
      }
    },
    {
      "content_type": "application/json",
      "content": {"key": "value"},
      "metadata": {}
    }
  ]
}
```

**支持的内容类型**:
- 文本 (text/plain, text/markdown)
- JSON (application/json)
- 二进制数据 (通过 content_url)
- 多模态内容（图片、音频等）

---

## 与 Claude Code 的对比

| 特性 | Claude Code | ACP 协议 |
|------|-------------|---------|
| **消息历史** | ✅ 通过 stdout 捕获 | ✅ 通过 API 获取 |
| **实时流式** | ✅ JSON 流 | ✅ SSE/WebSocket |
| **事件类型** | 7+ 种 | 10+ 种 |
| **会话管理** | ✅ session_id | ✅ Session API |
| **标准化** | ❌ 专有格式 | ✅ 开放标准 |
| **跨 Agent** | ❌ 仅 Claude | ✅ 任何 ACP Agent |

---

## ACP 获取消息历史的方式

### 方式 1: 轮询事件 API

```rust
// 定期轮询获取新事件
async fn poll_events(run_id: &str) -> Result<Vec<Event>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://acp-server/runs/{}/events", run_id))
        .send()
        .await?;

    let events: RunEventsListResponse = response.json().await?;
    Ok(events.events)
}

// 使用
loop {
    let events = poll_events(&run_id).await?;
    for event in events {
        process_event(event);
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### 方式 2: Server-Sent Events (SSE)

```rust
// 订阅 SSE 流
async fn subscribe_events(run_id: &str) -> Result<EventStream> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://acp-server/runs/{}/events/stream", run_id))
        .header("Accept", "text/event-stream")
        .send()
        .await?;

    let stream = response.bytes_stream();
    Ok(EventStream::new(stream))
}

// 使用
let mut stream = subscribe_events(&run_id).await?;
while let Some(event) = stream.next().await {
    match event.event_type.as_str() {
        "message.part" => {
            // 处理流式消息片段
            msg_store.push(LogMsg::from_acp_event(event));
        }
        "message.completed" => {
            // 消息完成
        }
        _ => {}
    }
}
```

### 方式 3: 获取会话历史

```rust
// 获取完整会话历史
async fn get_session_history(session_id: &str) -> Result<Vec<Message>> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://acp-server/session/{}", session_id))
        .send()
        .await?;

    let session: Session = response.json().await?;

    // 获取历史消息
    let mut messages = Vec::new();
    for history_uri in session.history {
        let msg = fetch_message(&history_uri).await?;
        messages.push(msg);
    }

    Ok(messages)
}
```

---

## OpenCode 的 ACP 实现

OpenCode 原生支持 ACP 协议，可以通过以下方式获取消息历史：

### 启动 ACP 服务器

```bash
# 启动 OpenCode ACP 服务器
opencode acp

# 默认监听 http://localhost:8001
```

### 连接并获取消息

```rust
// 1. 创建任务
let task_response = client
    .post("http://localhost:8001/runs")
    .json(&json!({
        "prompt": "add docstring to test.py"
    }))
    .send()
    .await?;

let run_id = task_response.json::<RunResponse>().await?.id;

// 2. 订阅事件流
let mut events = subscribe_events(&run_id).await?;

// 3. 实时接收消息
while let Some(event) = events.next().await {
    println!("Event: {:?}", event);

    // 存储到 MsgStore
    msg_store.push(LogMsg::from_acp_event(event));
}

// 4. 获取完整历史（任务完成后）
let history = get_session_history(&session_id).await?;
```

---

## 优势分析

### ACP 方案的优势

1. **标准化** ✅
   - 开放标准，Linux Foundation 支持
   - 任何 ACP 兼容的 Agent 都可以使用
   - 不依赖特定实现

2. **完整性** ✅
   - 会话级别的历史管理
   - 结构化的事件类型
   - 支持多模态内容

3. **实时性** ✅
   - SSE 实时推送
   - 增量更新（Delta）
   - 低延迟

4. **可扩展性** ✅
   - 分布式会话支持
   - 跨服务器状态同步
   - 支持长时间运行的任务

### 与 stdio 捕获的对比

| 维度 | stdio 捕获 | ACP 协议 |
|------|-----------|---------|
| **实现复杂度** | 简单 | 中等 |
| **标准化** | 否 | 是 |
| **跨 Agent** | 需要适配 | 统一接口 |
| **历史查询** | 需要自己存储 | 内置支持 |
| **实时性** | 优秀 | 优秀 |
| **可靠性** | 依赖进程 | 更可靠 |

---

## 推荐方案

### 对于 VibeRepo

**混合方案** - 根据 Agent 类型选择最佳方式：

```
┌─────────────────────────────────────┐
│      VibeRepo ExecutionService      │
└────────────┬────────────────────────┘
             │
    ┌────────┴────────┐
    │                 │
    ▼                 ▼
┌─────────┐    ┌──────────────┐
│ Claude  │    │ OpenCode     │
│ Code    │    │ (ACP)        │
│         │    │              │
│ stdio   │    │ GET /events  │
│ 捕获    │    │ SSE 订阅     │
└─────────┘    └──────────────┘
    │                 │
    └────────┬────────┘
             ▼
    ┌────────────────┐
    │   MsgStore     │
    │  (统一存储)    │
    └────────────────┘
```

**实现优先级**:

1. **Phase 1**: Claude Code (stdio 捕获)
   - 简单直接
   - 已验证可行
   - 快速实现

2. **Phase 2**: OpenCode (ACP 协议)
   - 标准化方案
   - 支持更多 Agents
   - 长期更好

---

## 结论

### ✅ ACP 完全支持消息历史获取

**核心能力**:
1. ✅ 会话历史 API (`GET /session/{id}`)
2. ✅ 事件流 API (`GET /runs/{id}/events`)
3. ✅ 实时流式传输 (SSE/WebSocket)
4. ✅ 结构化事件类型 (10+ 种)
5. ✅ 多模态内容支持

**对比 Claude Code**:
- ACP 更标准化
- ACP 支持跨 Agent
- ACP 有内置的历史管理
- Claude Code 更简单直接

**建议**:
- 短期：使用 Claude Code 的 stdio 捕获（快速实现）
- 长期：迁移到 ACP 协议（标准化、可扩展）

---

## 参考资料

- [ACP 官方文档](https://agentcommunicationprotocol.dev/)
- [ACP OpenAPI 规范](https://github.com/i-am-bee/acp/blob/main/docs/spec/openapi.yaml)
- [ACP GitHub](https://github.com/i-am-bee/acp)
- [BeeAI Framework](https://framework.beeai.dev/integrations/acp)

**Sources:**
- [Agent Communication Protocol](https://agentcommunicationprotocol.dev/)
- [ACP Explained - CodeStandUp](https://codestandup.com/posts/2025/agent-client-protocol-acp-explained/)
- [i-am-bee/acp GitHub](https://github.com/i-am-bee/acp)
- [The Foundational Languages of Agent Collaboration](https://re-cinq.com/blog/agents-in-dialogue-part-2-acps)
- [ACP Protocol Standard](https://www.gocodeo.com/post/acp-the-protocol-standard-for-ai-agent-interoperability)
