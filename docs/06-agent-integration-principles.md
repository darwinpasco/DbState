# Agent Integration Principles

DbState should eventually integrate well with Codex and Claude.

## Preferred surfaces

- CLI.
- Structured JSON output.
- Local MCP server.
- Repository guidance files such as `AGENTS.md`.
- Claude guidance files such as `CLAUDE.md`, if adopted.

## Boundaries

Agent workflows should be read-only or dry-run by default.

Agents may:

- Inspect repositories.
- Run dry-run comparisons.
- Explain schema and data diffs.
- Summarize selected synchronization plans.
- Review generated SQL scripts.
- Review risk and dependency reports.
- Draft branch names, commit messages, PR titles, PR bodies, and comments from deterministic artifacts.

Agents must not:

- Apply production changes by default.
- Execute generated synchronization scripts against real target databases.
- Receive raw credentials.
- Expose production data or unmasked PII.
- Generate DDL as the source of truth.
- Override deterministic risk or dependency classifications.

DbState's deterministic engine remains the authority for diffs, dependency analysis, risk classification, and script generation.
