# ADR 0005: Agent Integration Through CLI, JSON, and MCP

## Status

Accepted

## Context

Codex, Claude, and future agents can help users understand database changes, review generated scripts, and draft workflow text. Agents must not become the correctness layer.

## Decision

DbState will integrate with AI agents through CLI, structured JSON, and MCP, not through AI-dependent core logic.

## Consequences

Agent integrations can inspect deterministic artifacts, summarize changes, and draft review text. They must not receive raw credentials, expose PII, override deterministic warnings, or execute database deployments.

## Alternatives considered

- AI-first database planning.
- Direct browser-to-agent control of database operations.

## Open questions

- Whether MCP ships in v0.1 or later.
- Whether Claude guidance should live in `CLAUDE.md`.
