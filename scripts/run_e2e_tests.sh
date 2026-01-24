#!/bin/bash
# E2E Test Runner Script for VibeRepo

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}=== VibeRepo E2E Test Runner ===${NC}\n"

# Check prerequisites
echo "Checking prerequisites..."

# Check if backend is running
if ! curl -s http://localhost:3000/health > /dev/null 2>&1; then
    echo -e "${RED}❌ VibeRepo backend is not running at http://localhost:3000${NC}"
    echo "Please start the backend with: cd backend && cargo run"
    exit 1
fi
echo -e "${GREEN}✓${NC} Backend is running"

# Check Docker
if ! docker info > /dev/null 2>&1; then
    echo -e "${RED}❌ Docker is not running${NC}"
    echo "Please start Docker daemon"
    exit 1
fi
echo -e "${GREEN}✓${NC} Docker is running"

# Check Gitea connectivity
if ! curl -s -k https://gitea.devo.top:66/api/v1/version > /dev/null 2>&1; then
    echo -e "${YELLOW}⚠${NC}  Warning: Cannot connect to Gitea instance"
    echo "Tests may fail if Gitea is not accessible"
fi

echo ""

# Parse arguments
TEST_NAME="${1:-}"
EXTRA_ARGS="${@:2}"

cd backend

if [ -z "$TEST_NAME" ]; then
    echo "Running all E2E tests..."
    cargo test --test e2e -- --ignored --nocapture $EXTRA_ARGS
else
    echo "Running E2E test: $TEST_NAME..."
    cargo test --test e2e $TEST_NAME -- --ignored --nocapture $EXTRA_ARGS
fi

echo ""
echo -e "${GREEN}=== E2E Tests Complete ===${NC}"
