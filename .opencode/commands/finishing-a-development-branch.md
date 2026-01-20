---
description: Activates when tasks complete. Verifies tests, presents options (merge/PR/keep/discard), cleans up worktree.
agent: build
model: anthropic/claude-sonnet-4-5
---

use use_skill tool with skill_name: "superpowers:finishing-a-development-branch"
