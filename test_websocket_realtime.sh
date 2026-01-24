#!/bin/bash
# WebSocket 实时日志测试脚本

TASK_ID=23
WS_URL="ws://localhost:3000/api/tasks/${TASK_ID}/logs/stream"

echo "🧪 WebSocket 实时日志测试"
echo "=========================="
echo "Task ID: $TASK_ID"
echo "WebSocket URL: $WS_URL"
echo ""

# 检查是否安装了 websocat
if command -v websocat &> /dev/null; then
    echo "✅ 使用 websocat 进行测试"
    echo ""
    
    # 创建临时文件存储日志
    LOG_FILE="/tmp/ws_test_${TASK_ID}.log"
    
    # 启动 WebSocket 连接（后台运行）
    echo "📡 正在连接 WebSocket..."
    websocat "$WS_URL" > "$LOG_FILE" 2>&1 &
    WS_PID=$!
    
    # 等待连接建立
    sleep 2
    
    if ps -p $WS_PID > /dev/null; then
        echo "✅ WebSocket 连接成功 (PID: $WS_PID)"
        echo ""
        
        # 显示初始消息
        if [ -s "$LOG_FILE" ]; then
            echo "📨 收到初始消息:"
            cat "$LOG_FILE"
            echo ""
        fi
        
        # 执行任务
        echo "🚀 正在执行任务..."
        curl -s -X POST "http://localhost:3000/api/tasks/${TASK_ID}/execute" | jq '.'
        echo ""
        
        # 等待并显示日志
        echo "📋 实时日志输出:"
        echo "----------------------------------------"
        
        # 持续显示日志 30 秒
        for i in {1..30}; do
            if [ -s "$LOG_FILE" ]; then
                # 显示新增的日志
                tail -n 50 "$LOG_FILE"
            fi
            
            # 检查任务状态
            TASK_STATUS=$(curl -s "http://localhost:3000/api/tasks/${TASK_ID}" | jq -r '.task_status')
            
            if [ "$TASK_STATUS" = "completed" ] || [ "$TASK_STATUS" = "failed" ]; then
                echo ""
                echo "✅ 任务已完成，状态: $TASK_STATUS"
                break
            fi
            
            sleep 1
        done
        
        echo "----------------------------------------"
        echo ""
        
        # 停止 WebSocket 连接
        kill $WS_PID 2>/dev/null || true
        
        # 显示完整日志
        echo "📄 完整日志内容:"
        echo "========================================"
        cat "$LOG_FILE"
        echo "========================================"
        echo ""
        
        # 清理
        rm -f "$LOG_FILE"
        
        echo "✅ 测试完成"
    else
        echo "❌ WebSocket 连接失败"
        cat "$LOG_FILE"
        rm -f "$LOG_FILE"
        exit 1
    fi
    
elif command -v python3 &> /dev/null; then
    echo "✅ 使用 Python 进行测试"
    echo ""
    
    # 使用 Python 测试
    python3 - <<'PYTHON_SCRIPT'
import asyncio
import json
import sys
import signal
from datetime import datetime

try:
    import websockets
    import aiohttp
except ImportError:
    print("❌ 请安装依赖: pip install websockets aiohttp")
    sys.exit(1)

TASK_ID = 23
WS_URL = f"ws://localhost:3000/api/tasks/{TASK_ID}/logs/stream"
API_URL = f"http://localhost:3000/api/tasks/{TASK_ID}"

async def monitor_websocket():
    """监听 WebSocket 日志"""
    print(f"📡 正在连接 WebSocket: {WS_URL}")
    
    try:
        async with websockets.connect(WS_URL) as ws:
            print("✅ WebSocket 连接成功")
            print("")
            print("📋 实时日志输出:")
            print("----------------------------------------")
            
            while True:
                try:
                    message = await asyncio.wait_for(ws.recv(), timeout=1.0)
                    timestamp = datetime.now().strftime("%H:%M:%S")
                    
                    # 尝试解析 JSON
                    try:
                        data = json.loads(message)
                        msg_type = data.get('type', 'unknown')
                        
                        if msg_type == 'connected':
                            print(f"[{timestamp}] 🔌 已连接到任务 {data.get('task_id')}")
                        elif msg_type == 'log':
                            print(f"[{timestamp}] 📝 {data.get('message', message)}")
                        else:
                            print(f"[{timestamp}] 📨 {message}")
                    except json.JSONDecodeError:
                        print(f"[{timestamp}] 📨 {message}")
                        
                except asyncio.TimeoutError:
                    # 超时，继续等待
                    continue
                except websockets.exceptions.ConnectionClosed:
                    print("")
                    print("🔌 WebSocket 连接已关闭")
                    break
                    
    except Exception as e:
        print(f"❌ WebSocket 错误: {e}")

async def execute_task():
    """执行任务"""
    # 等待 WebSocket 连接建立
    await asyncio.sleep(2)
    
    print("")
    print("🚀 正在执行任务...")
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_URL}/execute") as resp:
                result = await resp.json()
                print(f"✅ 任务执行请求已发送")
                print("")
    except Exception as e:
        print(f"❌ 执行任务失败: {e}")

async def check_task_status():
    """定期检查任务状态"""
    await asyncio.sleep(3)
    
    for _ in range(60):  # 最多检查 60 秒
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(API_URL) as resp:
                    data = await resp.json()
                    status = data.get('task_status')
                    
                    if status in ['completed', 'failed']:
                        print("")
                        print(f"✅ 任务已完成，状态: {status}")
                        return
                        
        except Exception as e:
            pass
            
        await asyncio.sleep(1)

async def main():
    """主函数"""
    # 创建任务
    ws_task = asyncio.create_task(monitor_websocket())
    exec_task = asyncio.create_task(execute_task())
    status_task = asyncio.create_task(check_task_status())
    
    # 等待任务完成
    try:
        await asyncio.gather(ws_task, exec_task, status_task)
    except KeyboardInterrupt:
        print("\n⚠️  用户中断")
    
    print("")
    print("----------------------------------------")
    print("✅ 测试完成")

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n⚠️  测试被中断")
        sys.exit(0)
PYTHON_SCRIPT

else
    echo "❌ 未找到 websocat 或 python3"
    echo "请安装其中之一:"
    echo "  - websocat: brew install websocat"
    echo "  - python3: 系统自带或 brew install python3"
    exit 1
fi
