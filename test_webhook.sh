#!/bin/bash

# Webhook 测试脚本
# 用于测试 VibeRepo 的 webhook 处理功能

SECRET="test-webhook-secret-123"
WEBHOOK_URL="http://localhost:3000/api/webhooks/7"

# PR 合并 webhook payload
PAYLOAD='{
  "action": "closed",
  "number": 4,
  "pull_request": {
    "id": 4,
    "number": 4,
    "state": "closed",
    "merged": true,
    "merged_at": "2026-01-22T01:40:22+08:00",
    "title": "Add goodbye feature",
    "body": "This PR adds a goodbye.txt file.\n\nCloses #3",
    "head": {
      "ref": "issue-3-add-goodbye",
      "sha": "ab83482"
    },
    "base": {
      "ref": "main",
      "sha": "f846857"
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

echo "=== Webhook 测试 ==="
echo "Payload:"
echo "$PAYLOAD" | jq .
echo ""
echo "Signature: $SIGNATURE"
echo ""
echo "发送 webhook 请求..."
echo ""

# 发送 webhook 请求
curl -v -X POST \
  -H "Content-Type: application/json" \
  -H "X-Gitea-Event: pull_request" \
  -H "X-Gitea-Signature: $SIGNATURE" \
  -d "$PAYLOAD" \
  "$WEBHOOK_URL" 2>&1 | tail -30

echo ""
echo "=== 测试完成 ==="
