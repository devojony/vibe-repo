---
description: Activates after design approval. Creates isolated workspace on new branch, runs project setup, verifies clean test baseline.
agent: build
model: anthropic/claude-sonnet-4-5
---

use use_skill tool with skill_name: "superpowers:using-git-worktrees"

$ARGUMENTS
