#!/bin/bash
# Test script for per-repository provider migration

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
WORKTREE_DIR="/Users/devo/workspace/vibo-repo/.worktrees/per-repo-provider-config"
TEST_DB="/tmp/test_per_repo_provider_migration.db"

echo "=== Testing Per-Repository Provider Migration ==="
echo ""

# Clean up any existing test database
rm -f "$TEST_DB"
echo "✓ Cleaned up existing test database"

# Set environment variables for test
export DATABASE_URL="sqlite:${TEST_DB}?mode=rwc"
export DATABASE_MAX_CONNECTIONS=5
export SERVER_HOST="127.0.0.1"
export SERVER_PORT=3001
export GITHUB_TOKEN="test_token"
export GITHUB_BASE_URL="https://api.github.com"
export WEBHOOK_SECRET="test_secret"
export DEFAULT_AGENT_COMMAND="opencode"
export DEFAULT_AGENT_TIMEOUT=600
export DEFAULT_DOCKER_IMAGE="ubuntu:22.04"
export WORKSPACE_BASE_DIR="/tmp/test_workspaces"
export RUST_LOG="info"

echo "✓ Set environment variables"
echo ""

# Build the project
echo "Building project..."
cd "$WORKTREE_DIR"
cargo build --manifest-path backend/Cargo.toml --quiet
echo "✓ Build completed"
echo ""

# Run the application briefly to trigger migrations
echo "Running migrations..."
timeout 5 cargo run --manifest-path backend/Cargo.toml --quiet 2>&1 | grep -E "(migration|Migration|table)" || true
echo ""

# Check if database was created
if [ -f "$TEST_DB" ]; then
    echo "✓ Database file created: $TEST_DB"
else
    echo "✗ Database file not created"
    exit 1
fi

# Verify table structure using sqlite3
echo ""
echo "=== Verifying Table Structure ==="
echo ""

# Check if repositories table exists
echo "Checking repositories table..."
sqlite3 "$TEST_DB" "SELECT name FROM sqlite_master WHERE type='table' AND name='repositories';" | grep -q "repositories" && echo "✓ repositories table exists" || (echo "✗ repositories table missing" && exit 1)

# Check if repo_providers table was dropped
echo "Checking repo_providers table was dropped..."
PROVIDER_COUNT=$(sqlite3 "$TEST_DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='repo_providers';")
if [ "$PROVIDER_COUNT" -eq 0 ]; then
    echo "✓ repo_providers table dropped"
else
    echo "✗ repo_providers table still exists"
    exit 1
fi

# Check if webhook_configs table was dropped
echo "Checking webhook_configs table was dropped..."
WEBHOOK_COUNT=$(sqlite3 "$TEST_DB" "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='webhook_configs';")
if [ "$WEBHOOK_COUNT" -eq 0 ]; then
    echo "✓ webhook_configs table dropped"
else
    echo "✗ webhook_configs table still exists"
    exit 1
fi

# Get repositories table schema
echo ""
echo "=== Repositories Table Schema ==="
sqlite3 "$TEST_DB" ".schema repositories"

# Verify new provider fields exist
echo ""
echo "=== Verifying New Provider Fields ==="
SCHEMA=$(sqlite3 "$TEST_DB" ".schema repositories")

check_field() {
    local field=$1
    if echo "$SCHEMA" | grep -q "$field"; then
        echo "✓ Field exists: $field"
        return 0
    else
        echo "✗ Field missing: $field"
        return 1
    fi
}

check_field "provider_type"
check_field "provider_base_url"
check_field "access_token"
check_field "webhook_secret"

# Verify all expected fields exist
echo ""
echo "=== Verifying All Expected Fields ==="
EXPECTED_FIELDS=(
    "id"
    "name"
    "full_name"
    "clone_url"
    "default_branch"
    "branches"
    "provider_type"
    "provider_base_url"
    "access_token"
    "webhook_secret"
    "validation_status"
    "has_required_branches"
    "has_required_labels"
    "can_manage_prs"
    "can_manage_issues"
    "validation_message"
    "status"
    "has_workspace"
    "webhook_status"
    "agent_command"
    "agent_timeout"
    "agent_env_vars"
    "docker_image"
    "deleted_at"
    "created_at"
    "updated_at"
)

ALL_FIELDS_OK=true
for field in "${EXPECTED_FIELDS[@]}"; do
    if ! check_field "$field"; then
        ALL_FIELDS_OK=false
    fi
done

echo ""
if [ "$ALL_FIELDS_OK" = true ]; then
    echo "=== ✓ All Tests Passed ==="
    echo ""
    echo "Migration Summary:"
    echo "  - Dropped: webhook_configs table"
    echo "  - Dropped: repo_providers table"
    echo "  - Recreated: repositories table with embedded provider config"
    echo "  - New fields: provider_type, provider_base_url, access_token, webhook_secret"
    echo "  - Total fields: ${#EXPECTED_FIELDS[@]}"
    exit 0
else
    echo "=== ✗ Some Tests Failed ==="
    exit 1
fi
