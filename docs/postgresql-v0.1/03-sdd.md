# DbState PostgreSQL v0.1 SDD

## System Context

DbState PostgreSQL v0.1 is a local-first product for PostgreSQL database state management. It operates on local Git repositories and PostgreSQL databases. It produces per-object desired-state files and generated deployment artifacts.

## Components

- Browser UI.
- DbState Service.
- DbState Core Engine.
- PostgreSQL Adapter.
- Git integration layer.
- Credential handling boundary.
- CLI.
- Docker automation image.
- Future MCP surface.

## Responsibilities

### Browser UI

The browser UI is the presentation layer. It shows projects, object explorers, schema compare, data compare, Git workflows, review screens, reports, dependency warnings, risk summaries, and AI draft review screens.

It does not perform Git, database, credential, compare, or SQL generation work directly.

### DbState Service

DbState Service is the local work executor. It:

- Reads and writes local repository files.
- Performs Git operations.
- Connects to PostgreSQL databases.
- Retrieves or stores credentials safely.
- Runs schema inspection through the PostgreSQL adapter.
- Runs configured reference-data comparison.
- Generates synchronization plans.
- Generates SQL synchronization scripts and deployment artifacts.
- Exposes local API to the browser UI.
- Supports CLI and future MCP-compatible operations.

### Core Engine

The core engine provides deterministic:

- Object normalization.
- Dependency analysis.
- Schema diff.
- Data diff.
- Risk classification.
- SQL generation.
- Deployment artifact generation.

### PostgreSQL Adapter

The PostgreSQL adapter provides PostgreSQL-specific:

- Schema inspection.
- Object normalization.
- Dependency analysis.
- SQL generation.
- Risk classification rules.

### CLI

The CLI supports headless local automation, JSON output, CI/CD usage, and agent integration.

### Docker

The Docker image wraps service and CLI capabilities for headless automation and CI/CD. It has no browser UI requirement for MVP.

## Data Flows

### Source database to repository

1. User opens a project.
2. User connects to a PostgreSQL source database.
3. DbState Service inspects schema and configured reference data.
4. Core Engine compares source state to repository state.
5. User approves selected updates.
6. DbState Service writes per-object files and reference-data files.
7. User approves Git stage, commit, and push actions.

### Repository to target database planning

1. User connects to a PostgreSQL target database.
2. DbState Service inspects target state.
3. Core Engine compares repository desired state to target state.
4. User cherry-picks changes.
5. Core Engine analyzes dependencies and risk.
6. DbState blocks critical missing dependencies by default.
7. DbState generates SQL synchronization scripts and companion artifacts.
8. User reviews artifacts and deploys outside DbState.

## Git Integration

Git operations are performed by DbState Service. The browser UI presents Git state and confirmations. The CLI exposes equivalent workflows.

DbState must not auto-commit or auto-push. It must not commit secrets or local-only configuration.

## Synchronization Safety

DbState may generate SQL synchronization scripts for target PostgreSQL databases. It must not execute generated SQL against target PostgreSQL databases through UI, service API, CLI, Docker, MCP, or AI integrations.

## Deployment Artifact Generation

Generated scripts and reports are written to the local repository, recommended under `database/releases/`. They are visible in Git status and Git diff.

Artifacts may include SQL script, summary markdown, risk JSON, object-level summary, dependency warnings, and AI-reviewable context.

## Error Handling

Errors should be classified and shown with clear next steps:

- Connection failure.
- Credential failure.
- PostgreSQL permission failure.
- Unsupported object.
- Normalization failure.
- Dependency blocker.
- Dirty working tree warning.
- Git conflict.
- Artifact generation failure.

Errors must not expose secrets.

## Security Design

- Local-first by default.
- Credentials stay behind the service boundary.
- Browser UI does not receive raw credentials.
- AI agents do not receive raw credentials.
- Repository files and generated artifacts must not contain secrets, tokens, connection strings, local-only paths, unmasked PII, or transactional production data.

## Open Technical Decisions

- Exact Rust web framework.
- Exact UI framework.
- Exact credential store strategy.
- Exact Git implementation approach.
- Exact service API routes.
- Exact local service port strategy.
- Exact artifact naming convention.
- Exact Docker shipping timing.
