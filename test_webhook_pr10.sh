#!/bin/bash

# Webhook 测试脚本 - 测试 PR #10 合并
SECRET="test-webhook-secret-123"
WEBHOOK_URL="http://localhost:3000/api/webhooks/7"

# PR 合并 webhook payload for PR #10
PAYLOAD='{
  "action": "closed",
  "number": 10,
  "pull_request": {
    "id": 10,
    "number": 10,
    "state": "closed",
    "merged": true,
    "merged_at": "2026-01-22T01:58:00+08:00",
    "title": "Webhook E2E test",
    "body": "End-to-end webhook test.\n\nCloses #9",
    "head": {
      "ref": "issue-9-webhook-e2e",
      "sha": "2bed616"
    },
    "base": {
      "ref": "main",
      "sha": "8698f7c"
    }
  },
  "repository": {
    "id": 109,
    "name": "vibe-repo-test",
    "full_name": "code-agent/vibe-repo-test",
    "owner": {
      "id": 14,
      "login": "code-agent"
    }
  },
  "sender": {
    "id": 14,
    "login": "code-agent"
  }
}'

# 计算 HMAC-SHA256 签名
SIGNATURE=$(echo -n "$PAYLOAD" | openssl dgst -sha256 -hmac "$SECRET" | sed 's/^.* //')

echo "=== 测试 PR #10 合并 Webhook ==="
echo "Signature: $SIGNATURE"
echo ""

# 发送 webhook 请求
RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -H "X-Gitea-Event: pull_request" \
  -H "X-Gitea-Signature: $SIGNATURE" \
  -d "$PAYLOAD" \
  "$WEBHOOK_URL")

echo "Response: $RESPONSE"
echo ""

# 等待处理
sleep 2

# 检查任务状态
echo "检查任务状态..."
curl -s 'http://localhost:3000/api/tasks/5?workspace_id=3' | jq '{id, task_status, completed_at}'

# 检查 Issue 状态
echo ""
echo "检查 Issue 状态..."
curl -s -H "Authorization: token fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2" \
  https://gitea.devo.top:66/api/v1/repos/code-agent/vibe-repo-test/issues/9 | jq '{number, state}'
