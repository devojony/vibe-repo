#!/bin/bash

# Webhook 测试脚本 - 测试 PR #8 合并
SECRET="test-webhook-secret-123"
WEBHOOK_URL="http://localhost:3000/api/webhooks/7"

# PR 合并 webhook payload for PR #8
PAYLOAD='{
  "action": "closed",
  "number": 8,
  "pull_request": {
    "id": 8,
    "number": 8,
    "state": "closed",
    "merged": true,
    "merged_at": "2026-01-22T01:50:00+08:00",
    "title": "Add final test",
    "body": "Final webhook test.\n\nCloses #7",
    "head": {
      "ref": "issue-7-final-test",
      "sha": "dfacc54"
    },
    "base": {
      "ref": "main",
      "sha": "11b459e"
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

echo "=== 测试 PR #8 合并 Webhook ==="
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
curl -s 'http://localhost:3000/api/tasks/4?workspace_id=3' | jq '{id, task_status, completed_at}'
