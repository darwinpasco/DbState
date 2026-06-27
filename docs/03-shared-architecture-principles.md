# Shared Architecture Principles

DbState should have one shared architecture direction across editions.

## Principles

- Deterministic core: core comparison, normalization, dependency analysis, risk classification, SQL generation, and deployment artifact generation must be deterministic.
- Local-first service: DbState Service should run locally by default and perform repository, Git, database, credential, comparison, and artifact work.
- Browser UI as presentation layer: the UI shows object explorers, visual compare, Git workflows, review screens, and reports. It does not directly perform core database operations.
- Native RDBMS adapters: adapters handle engine-specific introspection, normalization, dependency analysis, and SQL generation.
- CLI: the CLI supports headless automation, JSON output, CI/CD, and agent integration.
- Docker: the Docker image supports headless automation, CI/CD, scheduled drift checks, and later server-style runtime.
- MCP later: a local MCP server may expose deterministic operations to agents later.
- Local-first security: core workflows should not require a cloud service.
- Consistent project format: each edition should share the same broad project structure where practical.
- No universal SQL DSL as core model: native SQL and native semantics are first-class.
- Cross-platform by architecture: DbState should run on Windows, macOS, and Linux.
- Separate native builds per platform: each RDBMS edition should ship native builds and CLI binaries for supported platforms.

## Preferred technology direction

- Rust core engine.
- Rust DbState Service, likely Axum or Actix Web.
- React plus TypeScript browser UI.
- Rust CLI.
- Docker image wrapping service and CLI.

These are preferred directions, not final decisions unless an ADR explicitly records them.
