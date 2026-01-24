#!/bin/bash
# WebSocket 快速测试脚本

set -e

echo "🧪 VibeRepo WebSocket 快速测试"
echo "=============================="
echo ""

# 颜色定义
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查依赖
check_dependency() {
    if ! command -v $1 &> /dev/null; then
        echo -e "${RED}❌ 未找到 $1，请先安装${NC}"
        echo "   安装方法: $2"
        exit 1
    fi
}

echo "1️⃣  检查依赖..."
check_dependency "curl" "系统自带"
check_dependency "jq" "brew install jq 或 apt-get install jq"

# 检查 websocat
if ! command -v websocat &> /dev/null; then
    echo -e "${YELLOW}⚠️  未找到 websocat，将使用 Python 脚本测试${NC}"
    USE_PYTHON=1
else
    echo -e "${GREEN}✅ websocat 已安装${NC}"
    USE_PYTHON=0
fi

# 检查后端
echo ""
echo "2️⃣  检查后端状态..."
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo -e "${RED}❌ 后端未运行${NC}"
    echo "   请先启动后端: cd backend && cargo run"
    exit 1
fi
echo -e "${GREEN}✅ 后端运行正常${NC}"

# 获取或创建测试任务
echo ""
echo "3️⃣  准备测试任务..."

# 尝试获取现有任务
EXISTING_TASK=$(curl -s "http://localhost:3000/api/tasks?limit=1" | jq -r '.tasks[0].id // empty')

if [ -n "$EXISTING_TASK" ]; then
    TASK_ID=$EXISTING_TASK
    echo -e "${GREEN}✅ 使用现有任务: ID=$TASK_ID${NC}"
else
    echo "   创建新任务..."
    
    # 检查是否有 workspace
    WORKSPACE_ID=$(curl -s "http://localhost:3000/api/workspaces?limit=1" | jq -r '.[0].id // empty')
    
    if [ -z "$WORKSPACE_ID" ]; then
        echo -e "${RED}❌ 没有可用的 workspace${NC}"
        echo "   请先创建 workspace"
        exit 1
    fi
    
    # 检查是否有 agent
    AGENT_ID=$(curl -s "http://localhost:3000/api/agents?workspace_id=$WORKSPACE_ID&limit=1" | jq -r '.[0].id // empty')
    
    if [ -z "$AGENT_ID" ]; then
        echo -e "${YELLOW}⚠️  没有可用的 agent，使用 workspace_id=$WORKSPACE_ID${NC}"
        AGENT_ID="null"
    fi
    
    # 创建任务
    RESPONSE=$(curl -s -X POST http://localhost:3000/api/tasks \
      -H "Content-Type: application/json" \
      -d "{
        \"workspace_id\": $WORKSPACE_ID,
        \"issue_number\": 999,
        \"issue_title\": \"WebSocket Test Task\",
        \"issue_body\": \"Automated WebSocket test\",
        \"priority\": \"high\",
        \"assigned_agent_id\": $AGENT_ID
      }")
    
    TASK_ID=$(echo $RESPONSE | jq -r '.id // empty')
    
    if [ -z "$TASK_ID" ] || [ "$TASK_ID" = "null" ]; then
        echo -e "${RED}❌ 创建任务失败${NC}"
        echo "   响应: $RESPONSE"
        exit 1
    fi
    
    echo -e "${GREEN}✅ 任务创建成功: ID=$TASK_ID${NC}"
fi

# 测试 WebSocket
echo ""
echo "4️⃣  测试 WebSocket 连接..."
echo "   端点: ws://localhost:3000/api/tasks/$TASK_ID/logs/stream"
echo ""

if [ $USE_PYTHON -eq 1 ]; then
    # 使用 Python 测试
    echo "   使用 Python 测试 (5秒超时)..."
    
    python3 - <<EOF
import asyncio
import sys

try:
    import websockets
except ImportError:
    print("❌ 请安装 websockets: pip install websockets")
    sys.exit(1)

async def test():
    uri = "ws://localhost:3000/api/tasks/$TASK_ID/logs/stream"
    try:
        async with websockets.connect(uri, open_timeout=5) as ws:
            print("✅ WebSocket 连接成功")
            
            # 接收连接消息
            msg = await asyncio.wait_for(ws.recv(), timeout=3)
            print(f"📨 收到消息: {msg}")
            
            return 0
    except asyncio.TimeoutError:
        print("⏱️  连接超时")
        return 1
    except Exception as e:
        print(f"❌ 连接失败: {e}")
        return 1

sys.exit(asyncio.run(test()))
EOF
    
    TEST_RESULT=$?
else
    # 使用 websocat 测试
    echo "   使用 websocat 测试 (5秒超时)..."
    
    timeout 5 websocat ws://localhost:3000/api/tasks/$TASK_ID/logs/stream > /tmp/ws_test_$$.log 2>&1 &
    WS_PID=$!
    
    sleep 2
    
    if ps -p $WS_PID > /dev/null 2>&1; then
        echo -e "${GREEN}✅ WebSocket 连接成功${NC}"
        
        # 显示收到的消息
        if [ -s /tmp/ws_test_$$.log ]; then
            echo "📨 收到的消息:"
            cat /tmp/ws_test_$$.log | head -5
        fi
        
        kill $WS_PID 2>/dev/null || true
        TEST_RESULT=0
    else
        echo -e "${RED}❌ WebSocket 连接失败${NC}"
        if [ -s /tmp/ws_test_$$.log ]; then
            echo "错误信息:"
            cat /tmp/ws_test_$$.log
        fi
        TEST_RESULT=1
    fi
    
    rm -f /tmp/ws_test_$$.log
fi

# 总结
echo ""
echo "=============================="
if [ $TEST_RESULT -eq 0 ]; then
    echo -e "${GREEN}🎉 WebSocket 测试通过！${NC}"
    echo ""
    echo "💡 提示:"
    echo "   - 手动测试: websocat ws://localhost:3000/api/tasks/$TASK_ID/logs/stream"
    echo "   - 执行任务: curl -X POST http://localhost:3000/api/tasks/$TASK_ID/execute"
    echo "   - 查看文档: docs/testing/websocket-testing.md"
else
    echo -e "${RED}❌ WebSocket 测试失败${NC}"
    echo ""
    echo "🔍 故障排查:"
    echo "   1. 检查后端日志: tail -f /tmp/vibe-repo-backend.log"
    echo "   2. 检查任务状态: curl http://localhost:3000/api/tasks/$TASK_ID"
    echo "   3. 查看文档: docs/testing/websocket-testing.md"
fi

exit $TEST_RESULT
