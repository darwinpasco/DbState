# PostgreSQL v0.1 Technical Architecture

## Local-First Runtime

DbState PostgreSQL v0.1 runs locally by default. Core workflows do not require a cloud service.

The local runtime includes DbState Service, browser UI, deterministic core engine, PostgreSQL adapter, Git integration, CLI, and optional Docker automation.

## Browser UI as Presentation Layer

The browser UI presents:

- Project open and clone.
- Connection setup.
- Object explorer.
- Visual schema compare.
- Visual data compare.
- Cherry-pick selection.
- Dependency warning display.
- Risk review.
- Script generation review.
- Generated artifact review.
- Git stage, commit, and push review.
- AI-assisted draft review.

The browser UI does not perform PostgreSQL operations, Git operations, credential operations, diff computation, risk classification, or SQL generation directly.

## DbState Service as Work Executor

DbState Service performs:

- Repository file reads and writes.
- Git operations.
- PostgreSQL database connections.
- Credential retrieval or storage through safe mechanisms.
- PostgreSQL schema inspection.
- Reference-data comparison.
- Synchronization plan generation.
- SQL synchronization script generation.
- Deployment artifact generation.
- Local API exposure to browser UI.
- Support for CLI and future MCP-compatible operations.

## Deterministic Core Engine

The core engine performs deterministic comparison, normalization, dependency analysis, risk classification, SQL generation, and artifact generation.

AI is not part of core correctness.

## PostgreSQL Adapter

The PostgreSQL adapter handles PostgreSQL-specific object inspection, normalization, dependency analysis, SQL generation, and risk rules.

It must not flatten PostgreSQL into a generic SQL model.

## Git Integration Layer

Git integration is handled by DbState Service. The first implementation approach is an open decision:

- System Git executable.
- libgit2/git2-rs.
- Hybrid.

DbState must support clone, open repository, status, branch, checkout, fetch, pull, stage, unstage, commit, and push with explicit user approval for state-changing operations.

## Credential Handling Boundary

Credentials must stay behind the service boundary. The browser UI and AI agents must not receive raw credentials.

Credential storage strategy is an open decision. Safe platform mechanisms are preferred.

## CLI

The CLI exposes headless workflows and JSON output for automation, CI/CD, and agent integration.

## Docker Image

The Docker image supports headless automation and CI/CD. It is not the primary desktop or human UI.

Whether Docker ships in v0.1 or immediately after remains an open decision.

## Future MCP Integration

MCP may expose deterministic local operations to agents later. It must preserve credential and no-direct-apply boundaries.

## Future Self-Hosted Server Path

A self-hosted DbState Server may support future Team or Enterprise workflows. It is not MVP.

## Cross-Platform Native Builds

DbState PostgreSQL should follow the general cross-platform strategy:

- Native Windows build.
- Native macOS build.
- Native Linux build.
- Native CLI binaries for Windows, macOS, and Linux.
- Docker image for Linux-based automation.

Packaging formats remain open decisions.
