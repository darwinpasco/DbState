# ADR PostgreSQL 0004: MVP No Direct Apply

## Status

Accepted

## Context

DbState PostgreSQL v0.1 must support planning and generated scripts while preserving the general DbState safety boundary.

## Decision

DbState PostgreSQL v0.1 will generate SQL synchronization scripts but will not directly apply them to target PostgreSQL databases.

## Consequences

DbState may synchronize local repository files from a source PostgreSQL database after user approval. It may compare repository desired state against a target PostgreSQL database and generate scripts, summaries, and risk reports.

DbState must not execute generated SQL against target PostgreSQL databases through browser UI, service API, CLI, Docker, MCP, or AI integrations.

Human-controlled deployment remains outside DbState.

## Alternatives considered

- Direct synchronization to target PostgreSQL databases.
- Optional apply button.
- Agent-executed deployment.

## Open questions

- How deployment instructions should reference external deployment tools.
- Exact wording of generated script safety headers.
