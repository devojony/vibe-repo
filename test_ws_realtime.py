#!/usr/bin/env python3
"""
WebSocket 实时日志测试
测试在任务执行过程中通过 WebSocket 获取实时日志
"""
import asyncio
import json
import sys
import os
from datetime import datetime

try:
    import websockets
except ImportError:
    print("❌ 请安装 websockets: pip3 install websockets")
    sys.exit(1)

try:
    import aiohttp
except ImportError:
    print("❌ 请安装 aiohttp: pip3 install aiohttp")
    sys.exit(1)

TASK_ID = 24
# 从环境变量读取 WebSocket 认证令牌
WS_AUTH_TOKEN = os.environ.get("WEBSOCKET_AUTH_TOKEN", "")
# 如果设置了令牌，添加到 URL 查询参数中
WS_URL = f"ws://localhost:3000/api/tasks/{TASK_ID}/logs/stream"
if WS_AUTH_TOKEN:
    WS_URL += f"?token={WS_AUTH_TOKEN}"
    print(f"🔐 使用认证令牌连接 WebSocket")
else:
    print(f"⚠️  未设置 WEBSOCKET_AUTH_TOKEN，使用无认证模式")

API_URL = f"http://localhost:3000/api/tasks/{TASK_ID}"

# 用于存储接收到的日志
received_logs = []

async def monitor_websocket():
    """监听 WebSocket 日志"""
    print(f"📡 正在连接 WebSocket...")
    print(f"   URL: {WS_URL}")
    print("")
    
    try:
        async with websockets.connect(WS_URL, open_timeout=10) as ws:
            print("✅ WebSocket 连接成功！")
            print("")
            print("📋 等待日志输出...")
            print("=" * 60)
            
            message_count = 0
            
            while True:
                try:
                    message = await asyncio.wait_for(ws.recv(), timeout=2.0)
                    timestamp = datetime.now().strftime("%H:%M:%S.%f")[:-3]
                    message_count += 1
                    
                    # 尝试解析 JSON
                    try:
                        data = json.loads(message)
                        msg_type = data.get('type', 'unknown')
                        
                        if msg_type == 'connected':
                            print(f"[{timestamp}] 🔌 已连接到任务 {data.get('task_id')}")
                            received_logs.append(f"Connected to task {data.get('task_id')}")
                        elif msg_type == 'log':
                            log_msg = data.get('message', message)
                            print(f"[{timestamp}] 📝 {log_msg}")
                            received_logs.append(log_msg)
                        else:
                            print(f"[{timestamp}] 📨 {json.dumps(data, ensure_ascii=False)}")
                            received_logs.append(str(data))
                    except json.JSONDecodeError:
                        print(f"[{timestamp}] 📨 {message}")
                        received_logs.append(message)
                        
                except asyncio.TimeoutError:
                    # 超时，检查是否应该继续
                    if message_count == 0:
                        # 如果一直没有收到消息，可能有问题
                        continue
                    else:
                        # 已经收到消息，继续等待
                        continue
                except websockets.exceptions.ConnectionClosed as e:
                    print("")
                    print(f"🔌 WebSocket 连接已关闭 (code: {e.code}, reason: {e.reason})")
                    break
                    
    except asyncio.TimeoutError:
        print("❌ WebSocket 连接超时")
        return False
    except Exception as e:
        print(f"❌ WebSocket 错误: {e}")
        import traceback
        traceback.print_exc()
        return False
    
    return True

async def execute_task():
    """执行任务"""
    # 等待 WebSocket 连接建立
    await asyncio.sleep(3)
    
    print("")
    print("🚀 正在执行任务...")
    print("")
    
    try:
        async with aiohttp.ClientSession() as session:
            async with session.post(f"{API_URL}/execute") as resp:
                if resp.status == 200:
                    result = await resp.json()
                    print(f"✅ 任务执行请求已发送")
                    print(f"   响应: {json.dumps(result, ensure_ascii=False, indent=2)}")
                else:
                    text = await resp.text()
                    print(f"❌ 任务执行失败 (HTTP {resp.status})")
                    print(f"   响应: {text}")
                print("")
    except Exception as e:
        print(f"❌ 执行任务失败: {e}")
        import traceback
        traceback.print_exc()

async def check_task_status():
    """定期检查任务状态"""
    await asyncio.sleep(5)
    
    print("🔍 开始监控任务状态...")
    print("")
    
    for i in range(120):  # 最多检查 120 秒
        try:
            async with aiohttp.ClientSession() as session:
                async with session.get(API_URL) as resp:
                    if resp.status == 200:
                        data = await resp.json()
                        status = data.get('task_status')
                        
                        if i % 10 == 0:  # 每 10 秒显示一次状态
                            print(f"   任务状态: {status}")
                        
                        if status in ['completed', 'failed', 'cancelled']:
                            print("")
                            print(f"✅ 任务已结束，最终状态: {status}")
                            
                            # 等待一下让 WebSocket 接收最后的日志
                            await asyncio.sleep(3)
                            return
                    else:
                        print(f"   获取任务状态失败: HTTP {resp.status}")
                        
        except Exception as e:
            print(f"   检查状态出错: {e}")
            
        await asyncio.sleep(1)
    
    print("")
    print("⏱️  超时：任务执行时间超过 120 秒")

async def main():
    """主函数"""
    print("=" * 60)
    print("🧪 WebSocket 实时日志测试")
    print("=" * 60)
    print(f"Task ID: {TASK_ID}")
    print(f"Workspace ID: 5")
    print("")
    
    # 创建任务
    ws_task = asyncio.create_task(monitor_websocket())
    exec_task = asyncio.create_task(execute_task())
    status_task = asyncio.create_task(check_task_status())
    
    # 等待任务完成
    try:
        done, pending = await asyncio.wait(
            [ws_task, exec_task, status_task],
            return_when=asyncio.FIRST_COMPLETED
        )
        
        # 取消未完成的任务
        for task in pending:
            task.cancel()
            try:
                await task
            except asyncio.CancelledError:
                pass
                
    except KeyboardInterrupt:
        print("\n⚠️  用户中断")
    
    print("")
    print("=" * 60)
    print("📊 测试结果")
    print("=" * 60)
    print(f"收到的日志消息数量: {len(received_logs)}")
    
    if len(received_logs) > 0:
        print("")
        print("前 10 条日志:")
        for i, log in enumerate(received_logs[:10], 1):
            # 截断过长的日志
            log_preview = log[:100] + "..." if len(log) > 100 else log
            print(f"  {i}. {log_preview}")
        
        if len(received_logs) > 10:
            print(f"  ... 还有 {len(received_logs) - 10} 条日志")
    
    print("")
    
    if len(received_logs) > 1:  # 至少有连接消息 + 1 条日志
        print("✅ 测试成功：WebSocket 能够接收实时日志！")
        return 0
    else:
        print("❌ 测试失败：未接收到足够的日志消息")
        return 1

if __name__ == "__main__":
    try:
        exit_code = asyncio.run(main())
        sys.exit(exit_code)
    except KeyboardInterrupt:
        print("\n⚠️  测试被中断")
        sys.exit(130)
