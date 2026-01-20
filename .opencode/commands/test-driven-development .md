---
description: Activates during implementation. Enforces RED-GREEN-REFACTOR: write failing test, watch it fail, write minimal code, watch it pass, commit. Deletes code written before tests.
agent: build
model: anthropic/claude-sonnet-4-5
---

use use_skill tool with skill_name: "superpowers:test-driven-development"
