#!/bin/bash

echo "=== 完整的 Webhook 端到端测试 ==="
echo ""

# 1. 创建 Issue
echo "1. 创建测试 Issue..."
ISSUE_RESPONSE=$(curl -s -X POST \
  -H "Authorization: token fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2" \
  -H "Content-Type: application/json" \
  -d '{
    "title": "Final E2E webhook test",
    "body": "Complete end-to-end webhook test."
  }' \
  https://gitea.devo.top:66/api/v1/repos/code-agent/vibe-repo-test/issues)

ISSUE_NUMBER=$(echo "$ISSUE_RESPONSE" | jq -r '.number')
echo "   ✓ Issue #$ISSUE_NUMBER 已创建"
echo ""

# 2. 创建分支和提交
echo "2. 创建分支和提交..."
cd /tmp/vibe-repo-test
git checkout main > /dev/null 2>&1
git pull > /dev/null 2>&1
git checkout -b "issue-${ISSUE_NUMBER}-final-e2e" > /dev/null 2>&1
echo "Final E2E test" > final-e2e.txt
git add final-e2e.txt
git commit -m "Add final-e2e.txt" > /dev/null 2>&1
git push https://code-agent:fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2@gitea.devo.top:66/code-agent/vibe-repo-test.git "issue-${ISSUE_NUMBER}-final-e2e" > /dev/null 2>&1
echo "   ✓ 分支 issue-${ISSUE_NUMBER}-final-e2e 已创建"
echo ""

# 3. 创建任务
echo "3. 创建任务..."
TASK_RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"workspace_id\": 3,
    \"issue_number\": $ISSUE_NUMBER,
    \"issue_title\": \"Final E2E webhook test\",
    \"issue_body\": \"Complete end-to-end webhook test.\",
    \"priority\": \"medium\"
  }" \
  http://localhost:3000/api/tasks)

TASK_ID=$(echo "$TASK_RESPONSE" | jq -r '.id')
echo "   ✓ Task #$TASK_ID 已创建"

# 更新任务状态
sqlite3 ./backend/data/vibe-repo/db/vibe-repo.db "UPDATE tasks SET branch_name = 'issue-${ISSUE_NUMBER}-final-e2e', task_status = 'in_progress' WHERE id = $TASK_ID;"
echo "   ✓ 任务状态已更新"
echo ""

# 4. 创建 PR
echo "4. 创建 PR..."
PR_RESPONSE=$(curl -s -X POST \
  -H "Content-Type: application/json" \
  -d "{
    \"title\": \"Final E2E webhook test\",
    \"body\": \"Complete end-to-end webhook test.\\n\\nCloses #$ISSUE_NUMBER\"
  }" \
  "http://localhost:3000/api/tasks/$TASK_ID/create-pr")

PR_NUMBER=$(echo "$PR_RESPONSE" | jq -r '.pr_number')
PR_URL=$(echo "$PR_RESPONSE" | jq -r '.pr_url')
echo "   ✓ PR #$PR_NUMBER 已创建"
echo "   URL: $PR_URL"
echo ""

# 5. 显示当前状态
echo "5. 当前状态："
TASK_STATUS=$(curl -s "http://localhost:3000/api/tasks/$TASK_ID?workspace_id=3" | jq '{task_status, pr_number}')
echo "   任务: $TASK_STATUS"
echo ""

# 6. 合并 PR
echo "6. 合并 PR #$PR_NUMBER..."
curl -s -X POST \
  -H "Authorization: token fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2" \
  -H "Content-Type: application/json" \
  -d '{"Do":"merge"}' \
  "https://gitea.devo.top:66/api/v1/repos/code-agent/vibe-repo-test/pulls/$PR_NUMBER/merge" > /dev/null
echo "   ✓ PR 已合并"
echo ""

# 7. 等待 webhook
echo "7. 等待 Gitea webhook 触发（10秒）..."
for i in {10..1}; do
  echo -n "   $i..."
  sleep 1
done
echo ""
echo ""

# 8. 检查最终状态
echo "8. 最终状态："
echo ""
echo "   任务状态:"
curl -s "http://localhost:3000/api/tasks/$TASK_ID?workspace_id=3" | jq '{id, task_status, completed_at}'
echo ""
echo "   Issue 状态:"
curl -s -H "Authorization: token fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2" \
  "https://gitea.devo.top:66/api/v1/repos/code-agent/vibe-repo-test/issues/$ISSUE_NUMBER" | jq '{number, state, closed_at}'
echo ""

# 9. 结果判断
FINAL_TASK_STATUS=$(curl -s "http://localhost:3000/api/tasks/$TASK_ID?workspace_id=3" | jq -r '.task_status')
FINAL_ISSUE_STATE=$(curl -s -H "Authorization: token fd784e3e2d498bb3d3f73d3b3db8d6d87d7737e2" \
  "https://gitea.devo.top:66/api/v1/repos/code-agent/vibe-repo-test/issues/$ISSUE_NUMBER" | jq -r '.state')

echo "=== 测试结果 ==="
echo ""
if [ "$FINAL_TASK_STATUS" = "completed" ] && [ "$FINAL_ISSUE_STATE" = "closed" ]; then
  echo "✅ 测试成功！"
  echo "   - Issue #$ISSUE_NUMBER: closed"
  echo "   - Task #$TASK_ID: completed"
  echo "   - Webhook 自动触发并正确处理"
else
  echo "⚠️  测试部分成功："
  echo "   - Issue #$ISSUE_NUMBER: $FINAL_ISSUE_STATE"
  echo "   - Task #$TASK_ID: $FINAL_TASK_STATUS"
  if [ "$FINAL_ISSUE_STATE" = "closed" ] && [ "$FINAL_TASK_STATUS" != "completed" ]; then
    echo ""
    echo "   Issue 已关闭但任务未更新，可能原因："
    echo "   1. Webhook 未被 Gitea 触发"
    echo "   2. Webhook URL 无法从 Gitea 访问"
    echo "   3. Webhook 处理时出现错误"
  fi
fi
echo ""
echo "测试完成！"
