# WebSocket 功能测试指南

本文档介绍如何测试 VibeRepo 的 WebSocket 实时日志流功能。

## 📋 目录

- [WebSocket 端点信息](#websocket-端点信息)
- [认证配置](#认证配置)
- [测试方法](#测试方法)
  - [方法 1: 使用 websocat (推荐)](#方法-1-使用-websocat-推荐)
  - [方法 2: 使用 wscat](#方法-2-使用-wscat)
  - [方法 3: 使用浏览器 JavaScript](#方法-3-使用浏览器-javascript)
  - [方法 4: 使用 Python 脚本](#方法-4-使用-python-脚本)
  - [方法 5: 使用 curl (HTTP 升级)](#方法-5-使用-curl-http-升级)
- [测试场景](#测试场景)
- [故障排查](#故障排查)

## 🔌 WebSocket 端点信息

### 端点 URL
```
ws://localhost:3000/api/tasks/{task_id}/logs/stream?token=YOUR_TOKEN
```

### 认证要求
- **认证方式**: Token-based authentication via query parameter
- **Token 参数**: `?token=YOUR_TOKEN`
- **配置**: 通过 `WEBSOCKET_AUTH_TOKEN` 环境变量设置
- **开发模式**: 如果未设置 `WEBSOCKET_AUTH_TOKEN`，认证将被禁用

### 功能说明
- **实时日志流**: 接收任务执行的实时日志输出
- **双向通信**: 支持 ping/pong 心跳检测
- **自动重连**: 客户端可以实现自动重连逻辑
- **安全认证**: 基于令牌的身份验证保护 WebSocket 连接

### 消息格式

**连接成功消息**:
```json
{
  "type": "connected",
  "task_id": 123
}
```

**日志消息**:
```json
{
  "type": "log",
  "task_id": 123,
  "timestamp": "2024-01-23T10:30:00Z",
  "level": "info",
  "message": "Task execution started"
}
```

## 🔐 认证配置

### 生成认证令牌

```bash
# 使用 openssl 生成安全的随机令牌
openssl rand -hex 32
```

### 配置环境变量

在 `.env` 文件中添加:
```bash
# WebSocket 认证令牌
WEBSOCKET_AUTH_TOKEN=your-generated-token-here
```

### 认证行为

| 场景 | 行为 |
|------|------|
| 未设置 `WEBSOCKET_AUTH_TOKEN` | 认证禁用，所有连接被接受 |
| 设置了 `WEBSOCKET_AUTH_TOKEN` | 需要提供正确的 token 才能连接 |
| Token 错误 | 返回 401 Unauthorized |
| Token 缺失 | 返回 401 Unauthorized |

### 安全建议

- ✅ **生产环境**: 必须设置 `WEBSOCKET_AUTH_TOKEN`
- ✅ **开发环境**: 可以不设置以简化测试
- ✅ **Token 管理**: 定期轮换令牌
- ✅ **传输安全**: 生产环境使用 WSS (WebSocket Secure)

## 🧪 测试方法

### 方法 1: 使用 websocat (推荐)

**安装 websocat**:
```bash
# macOS
brew install websocat

# Linux
cargo install websocat

# 或下载预编译二进制
# https://github.com/vi/websocat/releases
```

**基本测试**:
```bash
# 不带认证（开发环境）
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 带认证令牌
export WEBSOCKET_AUTH_TOKEN="your-token-here"
websocat "ws://localhost:3000/api/tasks/1/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"

# 带详细输出
websocat -v "ws://localhost:3000/api/tasks/1/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"

# 自动重连
websocat --ping-interval 30 "ws://localhost:3000/api/tasks/1/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"
```

**测试步骤**:
```bash
# 1. 设置认证令牌（如果需要）
export WEBSOCKET_AUTH_TOKEN="your-token-here"

# 2. 启动 VibeRepo 后端
cd backend
cargo run

# 3. 在另一个终端创建一个任务
curl -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 1,
    "issue_title": "Test task",
    "issue_body": "Test description",
    "priority": "high",
    "assigned_agent_id": 1
  }'

# 假设返回的 task_id 是 5

# 4. 连接 WebSocket（带认证）
websocat "ws://localhost:3000/api/tasks/5/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"

# 5. 在另一个终端执行任务
curl -X POST http://localhost:3000/api/tasks/5/execute

# 6. 观察 websocat 终端，应该能看到实时日志输出
```

### 方法 2: 使用 wscat

**安装 wscat**:
```bash
npm install -g wscat
```

**测试命令**:
```bash
# 不带认证（开发环境）
wscat -c ws://localhost:3000/api/tasks/1/logs/stream

# 带认证令牌
export WEBSOCKET_AUTH_TOKEN="your-token-here"
wscat -c "ws://localhost:3000/api/tasks/1/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"
```

### 方法 3: 使用浏览器 JavaScript

创建一个 HTML 文件 `websocket-test.html`:

```html
<!DOCTYPE html>
<html>
<head>
    <title>VibeRepo WebSocket Test</title>
    <style>
        body {
            font-family: monospace;
            padding: 20px;
            background: #1e1e1e;
            color: #d4d4d4;
        }
        #logs {
            background: #252526;
            padding: 15px;
            border-radius: 5px;
            height: 500px;
            overflow-y: auto;
            white-space: pre-wrap;
            word-wrap: break-word;
        }
        .controls {
            margin-bottom: 20px;
        }
        input, button {
            padding: 8px 12px;
            margin-right: 10px;
            font-size: 14px;
        }
        .connected { color: #4ec9b0; }
        .disconnected { color: #f48771; }
        .log-message { color: #ce9178; }
        .error { color: #f48771; }
    </style>
</head>
<body>
    <h1>VibeRepo WebSocket 测试工具</h1>
    
    <div class="controls">
        <label>Task ID: <input type="number" id="taskId" value="1" /></label>
        <label>Token: <input type="text" id="token" placeholder="认证令牌（可选）" size="40" /></label>
        <button onclick="connect()">连接</button>
        <button onclick="disconnect()">断开</button>
        <button onclick="clearLogs()">清空日志</button>
        <span id="status" class="disconnected">未连接</span>
    </div>
    
    <div id="logs"></div>

    <script>
        let ws = null;
        const logsDiv = document.getElementById('logs');
        const statusSpan = document.getElementById('status');

        function log(message, className = '') {
            const timestamp = new Date().toISOString();
            const line = document.createElement('div');
            line.className = className;
            line.textContent = `[${timestamp}] ${message}`;
            logsDiv.appendChild(line);
            logsDiv.scrollTop = logsDiv.scrollHeight;
        }

        function connect() {
            const taskId = document.getElementById('taskId').value;
            const token = document.getElementById('token').value;
            
            // 构建 URL，如果提供了 token 则添加到查询参数
            let url = `ws://localhost:3000/api/tasks/${taskId}/logs/stream`;
            if (token) {
                url += `?token=${encodeURIComponent(token)}`;
            }
            
            log(`正在连接到: ${url}`);
            
            try {
                ws = new WebSocket(url);
                
                ws.onopen = () => {
                    log('✅ WebSocket 连接已建立', 'connected');
                    statusSpan.textContent = '已连接';
                    statusSpan.className = 'connected';
                };
                
                ws.onmessage = (event) => {
                    try {
                        const data = JSON.parse(event.data);
                        log(`📨 收到消息: ${JSON.stringify(data, null, 2)}`, 'log-message');
                    } catch (e) {
                        log(`📨 收到消息: ${event.data}`, 'log-message');
                    }
                };
                
                ws.onerror = (error) => {
                    log(`❌ WebSocket 错误: ${error}`, 'error');
                };
                
                ws.onclose = (event) => {
                    if (event.code === 1008) {
                        log(`🔌 WebSocket 连接已关闭: 认证失败 (code: ${event.code})`, 'error');
                    } else {
                        log(`🔌 WebSocket 连接已关闭 (code: ${event.code}, reason: ${event.reason})`, 'disconnected');
                    }
                    statusSpan.textContent = '未连接';
                    statusSpan.className = 'disconnected';
                };
                
            } catch (error) {
                log(`❌ 连接失败: ${error}`, 'error');
            }
        }

        function disconnect() {
            if (ws) {
                ws.close();
                ws = null;
            }
        }

        function clearLogs() {
            logsDiv.innerHTML = '';
        }

        // 页面加载时自动连接
        // window.onload = connect;
    </script>
</body>
</html>
```

**使用方法**:
1. 保存上述 HTML 文件
2. 在浏览器中打开
3. 输入 Task ID 和认证令牌（如果需要）
4. 点击"连接"按钮
5. 在另一个终端执行任务，观察实时日志

### 方法 4: 使用 Python 脚本

项目根目录已包含 `test_ws_realtime.py` 脚本，支持认证令牌。

**使用方法**:
```bash
# 安装依赖
pip install websockets aiohttp

# 设置认证令牌（如果需要）
export WEBSOCKET_AUTH_TOKEN="your-token-here"

# 运行测试脚本
python test_ws_realtime.py
```

**自定义测试脚本**:
```python
#!/usr/bin/env python3
"""
VibeRepo WebSocket 测试脚本（带认证）
"""
import asyncio
import websockets
import json
import sys
import os

async def test_websocket(task_id: int, token: str = None):
    # 构建 URL
    uri = f"ws://localhost:3000/api/tasks/{task_id}/logs/stream"
    if token:
        uri += f"?token={token}"
    
    print(f"正在连接到: {uri}")
    
    try:
        async with websockets.connect(uri) as websocket:
            print("✅ WebSocket 连接已建立")
            
            # 接收消息
            while True:
                try:
                    message = await websocket.recv()
                    
                    # 尝试解析 JSON
                    try:
                        data = json.loads(message)
                        print(f"📨 收到消息: {json.dumps(data, indent=2, ensure_ascii=False)}")
                    except json.JSONDecodeError:
                        print(f"📨 收到消息: {message}")
                        
                except websockets.exceptions.ConnectionClosed as e:
                    if e.code == 1008:
                        print("❌ 连接被拒绝: 认证失败")
                    else:
                        print(f"🔌 连接已关闭 (code: {e.code})")
                    break
                    
    except Exception as e:
        print(f"❌ 错误: {e}")
        return 1
    
    return 0

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("用法: python test_websocket.py <task_id> [token]")
        sys.exit(1)
    
    task_id = int(sys.argv[1])
    token = sys.argv[2] if len(sys.argv) > 2 else os.environ.get("WEBSOCKET_AUTH_TOKEN")
    
    exit_code = asyncio.run(test_websocket(task_id, token))
    sys.exit(exit_code)
```

### 方法 5: 使用 curl (HTTP 升级)

```bash
# 测试 WebSocket 握手（不带认证）
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
  http://localhost:3000/api/tasks/1/logs/stream

# 测试 WebSocket 握手（带认证）
curl -i -N \
  -H "Connection: Upgrade" \
  -H "Upgrade: websocket" \
  -H "Sec-WebSocket-Version: 13" \
  -H "Sec-WebSocket-Key: x3JJHMbDL1EzLkh9GBhXDw==" \
  "http://localhost:3000/api/tasks/1/logs/stream?token=your-token-here"
```

注意: curl 只能测试握手，不能接收 WebSocket 消息。

## 🧪 测试场景

### 场景 1: 基本连接测试

**目标**: 验证 WebSocket 连接能够建立

```bash
# 1. 启动后端
cargo run

# 2. 创建测试任务
TASK_ID=$(curl -s -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 1,
    "issue_title": "Test",
    "issue_body": "Test",
    "priority": "high",
    "assigned_agent_id": 1
  }' | jq -r '.id')

echo "Created task ID: $TASK_ID"

# 3. 连接 WebSocket
websocat ws://localhost:3000/api/tasks/$TASK_ID/logs/stream
```

**预期结果**:
- 连接成功
- 收到 `{"type":"connected","task_id":...}` 消息

### 场景 2: 实时日志流测试

**目标**: 验证任务执行时能接收实时日志

```bash
# 终端 1: 连接 WebSocket
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 终端 2: 执行任务
curl -X POST http://localhost:3000/api/tasks/1/execute
```

**预期结果**:
- 终端 1 实时显示任务执行日志
- 日志包含任务状态变化、命令输出等

### 场景 3: 多客户端连接测试

**目标**: 验证多个客户端可以同时连接

```bash
# 终端 1
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 终端 2
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 终端 3
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 终端 4: 执行任务
curl -X POST http://localhost:3000/api/tasks/1/execute
```

**预期结果**:
- 所有终端都能接收到相同的日志消息
- 广播机制正常工作

### 场景 4: 连接断开重连测试

**目标**: 验证客户端断开后可以重新连接

```bash
# 1. 连接
websocat ws://localhost:3000/api/tasks/1/logs/stream

# 2. 按 Ctrl+C 断开

# 3. 重新连接
websocat ws://localhost:3000/api/tasks/1/logs/stream
```

**预期结果**:
- 重新连接成功
- 收到新的连接消息

### 场景 5: 不存在的任务测试

**目标**: 验证错误处理

```bash
# 连接不存在的任务
websocat ws://localhost:3000/api/tasks/99999/logs/stream
```

**预期结果**:
- 连接被拒绝或立即关闭
- 返回错误信息

### 场景 6: 长时间连接测试

**目标**: 验证连接稳定性

```bash
# 保持连接 10 分钟
timeout 600 websocat --ping-interval 30 \
  ws://localhost:3000/api/tasks/1/logs/stream
```

**预期结果**:
- 连接保持稳定
- Ping/Pong 心跳正常

## 🔍 故障排查

### 问题 1: 连接被拒绝

**症状**:
```
Error: Connection refused
```

**解决方法**:
1. 检查后端是否运行: `curl http://localhost:3000/health`
2. 检查端口是否正确: 默认 3000
3. 检查防火墙设置

### 问题 2: 认证失败 (401 Unauthorized)

**症状**:
```
HTTP/1.1 401 Unauthorized
Invalid authentication token
```
或
```
Missing authentication token
```

**解决方法**:
1. 检查是否设置了 `WEBSOCKET_AUTH_TOKEN` 环境变量
2. 确认提供的 token 与配置的 token 一致
3. 检查 token 是否正确编码在 URL 中: `?token=YOUR_TOKEN`
4. 开发环境可以临时禁用认证（不设置 `WEBSOCKET_AUTH_TOKEN`）

**示例**:
```bash
# 查看当前配置的 token
echo $WEBSOCKET_AUTH_TOKEN

# 使用正确的 token 连接
websocat "ws://localhost:3000/api/tasks/1/logs/stream?token=${WEBSOCKET_AUTH_TOKEN}"
```

### 问题 3: 任务不存在

**症状**:
```
Error: Task not found
```

**解决方法**:
1. 检查任务 ID 是否正确
2. 查询任务列表: `curl http://localhost:3000/api/tasks`
3. 创建新任务进行测试

### 问题 4: 没有收到日志

**症状**:
- 连接成功但没有日志输出

**解决方法**:
1. 确认任务正在执行: `curl http://localhost:3000/api/tasks/{id}`
2. 检查任务状态是否为 "running"
3. 执行任务: `curl -X POST http://localhost:3000/api/tasks/{id}/execute`
4. 检查后端日志: `tail -f /tmp/vibe-repo-backend.log`

### 问题 5: 连接频繁断开

**症状**:
- 连接建立后很快断开

**解决方法**:
1. 启用 ping/pong: `websocat --ping-interval 30 ...`
2. 检查网络稳定性
3. 检查后端日志是否有错误
4. 确认认证 token 有效

### 问题 6: CORS 错误 (浏览器)

**症状**:
```
Access to WebSocket blocked by CORS policy
```

**解决方法**:
1. 检查后端 CORS 配置
2. 使用相同域名访问
3. 或使用命令行工具测试

### 问题 7: Token 在 URL 中被截断

**症状**:
- Token 包含特殊字符导致解析错误

**解决方法**:
1. 确保 token 正确 URL 编码
2. 使用简单的十六进制字符串作为 token（推荐）
3. 在 JavaScript 中使用 `encodeURIComponent(token)`

**示例**:
```javascript
// 正确的 URL 编码
const token = "my-token-with-special-chars!@#";
const url = `ws://localhost:3000/api/tasks/1/logs/stream?token=${encodeURIComponent(token)}`;
```

## 📊 监控和调试

### 查看后端日志

```bash
# 实时查看日志
tail -f /tmp/vibe-repo-backend.log | grep -i websocket

# 或使用 RUST_LOG
RUST_LOG=debug,vibe_repo=trace cargo run
```

### 查看 WebSocket 连接统计

```bash
# 查看活跃的 WebSocket 连接数
# (需要实现相应的 API 端点)
curl http://localhost:3000/api/stats/websockets
```

### 使用浏览器开发者工具

1. 打开浏览器开发者工具 (F12)
2. 切换到 "Network" 标签
3. 过滤 "WS" (WebSocket)
4. 查看 WebSocket 连接详情、消息等

## 📝 测试清单

使用以下清单确保 WebSocket 功能完全正常：

- [ ] 基本连接测试通过
- [ ] 能接收连接成功消息
- [ ] 能接收实时日志消息
- [ ] 多客户端同时连接正常
- [ ] 断开重连功能正常
- [ ] 不存在的任务返回错误
- [ ] Ping/Pong 心跳正常
- [ ] 长时间连接稳定
- [ ] 任务完成后连接正常关闭
- [ ] 错误处理正确

## 🚀 自动化测试脚本

创建 `test_websocket_full.sh`:

```bash
#!/bin/bash
set -e

echo "🧪 VibeRepo WebSocket 完整测试"
echo "================================"

# 检查后端是否运行
echo "1. 检查后端状态..."
if ! curl -s http://localhost:3000/health > /dev/null; then
    echo "❌ 后端未运行，请先启动: cargo run"
    exit 1
fi
echo "✅ 后端运行正常"

# 创建测试任务
echo "2. 创建测试任务..."
TASK_ID=$(curl -s -X POST http://localhost:3000/api/tasks \
  -H "Content-Type: application/json" \
  -d '{
    "workspace_id": 1,
    "issue_number": 999,
    "issue_title": "WebSocket Test Task",
    "issue_body": "Automated test",
    "priority": "high",
    "assigned_agent_id": 1
  }' | jq -r '.id')

if [ -z "$TASK_ID" ] || [ "$TASK_ID" = "null" ]; then
    echo "❌ 创建任务失败"
    exit 1
fi
echo "✅ 任务创建成功: ID=$TASK_ID"

# 测试 WebSocket 连接
echo "3. 测试 WebSocket 连接..."
timeout 5 websocat ws://localhost:3000/api/tasks/$TASK_ID/logs/stream > /tmp/ws_test.log 2>&1 &
WS_PID=$!
sleep 2

if ps -p $WS_PID > /dev/null; then
    echo "✅ WebSocket 连接成功"
    kill $WS_PID 2>/dev/null || true
else
    echo "❌ WebSocket 连接失败"
    cat /tmp/ws_test.log
    exit 1
fi

# 清理
echo "4. 清理测试数据..."
curl -s -X DELETE http://localhost:3000/api/tasks/$TASK_ID > /dev/null
echo "✅ 清理完成"

echo ""
echo "🎉 所有测试通过！"
```

**运行测试**:
```bash
chmod +x test_websocket_full.sh
./test_websocket_full.sh
```

## 📚 相关文档

- [WebSocket 实现代码](../backend/src/api/tasks/websocket.rs)
- [API 参考文档](./api/api-reference.md)
- [开发指南](./development/README.md)

## 🆘 获取帮助

如果遇到问题：
1. 查看后端日志
2. 检查 [故障排查](#故障排查) 部分
3. 在 GitHub Issues 中搜索类似问题
4. 创建新的 Issue 并提供详细信息
