# DbState PostgreSQL v0.1 Overview

DbState PostgreSQL v0.1 is the first RDBMS-specific edition of DbState. It applies the general DbState product thesis to PostgreSQL without turning PostgreSQL into a generic SQL abstraction.

## Purpose

The purpose of v0.1 is to prove the core DbState workflow for PostgreSQL:

- Git is the source of truth.
- One durable PostgreSQL object equals one file.
- The repository stores desired PostgreSQL database state.
- DbState can inspect a PostgreSQL database and synchronize selected objects into local repository files.
- DbState can compare repository state against a target PostgreSQL database.
- DbState can generate version-controlled SQL synchronization scripts and review artifacts.
- DbState must not directly apply generated SQL to target PostgreSQL databases.

## Scope

DbState PostgreSQL v0.1 covers local-first workflows for one user or local automation:

- Browser UI as presentation layer.
- Local DbState Service as the work executor.
- Deterministic core engine.
- Native PostgreSQL adapter.
- First-class Git workflows.
- Per-object desired-state files.
- Visual schema compare.
- Basic configured reference-data compare.
- Dependency-aware cherry-picking.
- Risk classification.
- CLI with JSON output.
- Docker image for headless automation.
- AI-assisted review text that is draft-only and grounded in DbState artifacts.

## Relationship to the General Foundation

This documentation pack extends the general DbState foundation documents. It does not replace them.

The PostgreSQL edition follows the foundation decisions:

- State-based, per-object database version control.
- Product family with native RDBMS editions.
- Cross-platform native builds.
- Local-first deterministic core.
- Agent integration through deterministic CLI, JSON, and future MCP surfaces.
- First-class Git integration.
- Generate SQL, do not apply SQL to target databases.
- Version-controlled deployment artifacts.
- Selective synchronization with dependency analysis.
- Deployment Rehearsal as premium later, not MVP.
- AI-assisted Git and review text as draft-only assistance.

## MVP Boundary

PostgreSQL v0.1 is documentation-scoped here. It defines product requirements and design direction. It does not implement code.

The MVP includes PostgreSQL only. It does not include MySQL, SQL Server, SQLite, Db2, or Access support.

## Non-Goals

- No Rust, React, Docker, CLI, service, API, or adapter scaffolding.
- No fork or separate repository.
- No direct target-database apply.
- No production deployment execution.
- No cloud dashboard.
- No self-hosted team server.
- No RBAC, SSO, policy gates, or compliance evidence packs.
- No Deployment Rehearsal in MVP.
- No full transactional data compare.

## Document Map

- `01-brd.md`: business requirements.
- `02-prd.md`: product requirements.
- `03-sdd.md`: software design document.
- `04-mvp-scope.md`: MVP scope and acceptance scenario.
- `05-technical-architecture.md`: technical architecture direction.
- `06-postgresql-object-model.md`: PostgreSQL object model.
- `07-reference-data-model.md`: controlled reference-data model.
- `08-git-workflows.md`: Git workflows.
- `09-synchronization-workflows.md`: synchronization and planning workflows.
- `10-cli-and-service-api.md`: CLI and service API design surface.
- `11-ui-workflows.md`: browser UI workflows.
- `12-security-and-safety.md`: security and safety model.
- `13-ai-agent-workflows.md`: Codex and Claude workflow boundaries.
- `14-commercial-packaging-alignment.md`: future tier alignment.
- `15-open-decisions.md`: unresolved decisions.
- `16-glossary.md`: PostgreSQL v0.1 glossary.
