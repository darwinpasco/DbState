# DbState

DbState is a product family for state-based, per-object database version control, schema compare, data compare, drift detection, and safe deployment planning.

DbState treats Git as the source of truth for desired database state. It stores durable database objects as files, generates reviewed deployment artifacts, and makes drift between Git and live databases visible, explainable, and actionable.

## Why DbState exists

Traditional database compare tools are useful because they show database differences visually and can generate synchronization scripts. Migration-only workflows are useful because they are explicit and reviewable. DbState aims to combine those strengths with a Git-native desired-state model.

The product should feel like a modern, Git-native successor to schema and data compare tools, while keeping the repository as the source of truth.

## Product family vision

DbState is not a universal lowest-common-denominator database abstraction. The long-term strategy is:

- One shared product philosophy.
- One shared architecture direction.
- One shared UX and automation model.
- Separate native RDBMS editions when the time is right.
- Cross-platform delivery for each RDBMS edition.
- Separate native builds per operating system.

Planned future editions:

- DbState PostgreSQL, planned first.
- DbState MySQL.
- DbState SQL Server.
- DbState SQLite.
- DbState Db2.
- DbState Access.

Detailed PostgreSQL v0.1 BRD, PRD, SDD, and implementation design documents are not created yet.

## Cross-platform strategy

DbState must be cross-platform by architecture and shipped as separate native builds per platform.

- Windows, macOS, and Linux are distribution targets.
- Operating systems are not separate product lines.
- Each RDBMS edition should have native builds and native CLI binaries for Windows, macOS, and Linux.
- Docker is for headless automation, CI/CD, scheduled drift checks, and later server-style runtime.

## Architecture direction

DbState uses a browser-based UI as the primary presentation layer. The browser UI does not perform core database operations directly.

Actual work is performed by:

- DbState Service, a local-first backend that reads and writes repository files, performs Git operations, connects to databases, stores or retrieves credentials safely, runs comparisons, generates plans, and exposes a local API.
- DbState Core Engine, a deterministic engine for normalization, diffing, dependency analysis, risk classification, SQL generation, and deployment artifact generation.
- Native RDBMS adapters for each database engine.
- CLI for automation, JSON output, CI/CD, and agent integration.
- Docker image for headless automation.

Preferred technology direction is Rust for the core engine, service, and CLI, likely Axum or Actix Web for the service, and React plus TypeScript for the browser UI. These are preferred directions, not final decisions.

## First-class Git integration

DbState should operate directly against Git repositories through the DbState Service, browser UI, CLI, and automation workflows.

Supported workflows should include clone, open repository, initialize project structure, status, branch, checkout, fetch, pull, stage, unstage, commit, push, object diff, merge-conflict detection, and dirty-working-tree warnings.

DbState must not auto-commit or auto-push without explicit user approval. DbState must not commit secrets, credentials, tokens, passwords, local-only configuration, local paths, or unmasked PII.

## Repository structure direction

Preferred DbState project structure:

```text
database/
  objects/
    schemas/
    extensions/
    enums/
    sequences/
    tables/
    indexes/
    views/
    materialized-views/
    functions/
    triggers/
    grants/
  reference-data/
    dbstate.reference-data.yml
    tables/
  releases/
```

`database/objects/` contains desired-state database object definitions. Schemas are first-class database objects under `database/objects/schemas/`. Other object files should use schema-qualified file names such as `core.payment_attempts.sql`.

## Reference data

Reference data should be configured explicitly under `database/reference-data/`. DbState must not assume all table data is version-controlled.

The registry file `database/reference-data/dbstate.reference-data.yml` should define controlled reference tables, business keys, ignored columns, masked columns, compare mode, and delete behavior.

Each controlled reference table should have one structured state file under `database/reference-data/tables/`, such as `public.payment_methods.yml`. YAML or JSON are preferred over raw SQL as the source of truth.

## Synchronization safety

DbState may synchronize a local repository from a source database after user approval. This can update object files and configured reference-data files.

When the local repository is ahead of a target database, DbState may compare, generate an object-level diff, allow cherry-picking, generate a SQL synchronization script, generate a risk report, and generate a deployment summary.

DbState must not directly apply generated SQL to a target database. This rule applies to browser UI, DbState Service, service API, CLI, Docker, MCP, future APIs, Codex, Claude, and other AI agents.

## Deployment artifacts

Generated synchronization scripts are version-controlled deployment artifacts. They are not the primary source of truth. Per-object state files remain the desired-state source of truth.

Generated scripts should be written into the repository by default, recommended under `database/releases/`. They must appear in Git status and Git diff. They may be staged, committed, and pushed only with explicit user approval.

Generated artifacts must not contain secrets, credentials, connection strings, tokens, local-only paths, unmasked PII, or transactional production data.

## Current status

This repository contains general DbState product-family foundation documents, PostgreSQL v0.1 planning documents, and the first narrow CLI implementation slices.

Current CLI scope:

- `dbstate repo status` reports Git and DbState project structure status.
- `dbstate init` initializes missing DbState PostgreSQL project folders and the safe empty reference-data registry.
- `dbstate inspect postgres` performs read-only PostgreSQL catalog inspection for schemas, tables, and columns.
- `dbstate export postgres` exports selected inspected schemas and simple ordinary tables to desired-state files under `database/objects/`.
- `dbstate sync postgres` compares selected source PostgreSQL objects against local desired-state files and creates or updates local files after explicit command invocation.
- `dbstate compare postgres` reads supported desired-state files, inspects PostgreSQL read-only, and reports repository-to-database differences without writing files.
- `dbstate plan postgres` builds an in-memory selected plan from compare results and reports limited table-to-schema dependency warnings without writing files.
- `dbstate release postgres` generates review-only SQL, summary, and risk artifacts under `database/releases/` from the selected plan. It never executes the generated SQL.
- `dbstate data-compare postgres` compares explicitly configured reference-data table files against PostgreSQL rows with read-only `SELECT`.
- CLI JSON output is hardened around common `command`, `success`, `warnings`, and `errors` fields, repository context where relevant, and redaction of connection details and masked values.
- Docker packaging is available for headless CLI automation against mounted repositories.

Current implementation limitations:

- No SQL execution, direct target-database apply, reference-data DML generation, arbitrary transactional data compare, browser UI, Docker Compose, CI workflow, MCP, or AI integration exists yet.
- PostgreSQL access is read-only in product commands.
- No command applies SQL to a database.

See `docs/postgresql-v0.1/28-slice-1-implementation-notes.md`, `docs/postgresql-v0.1/29-slice-2-implementation-notes.md`, `docs/postgresql-v0.1/30-slice-3-implementation-notes.md`, `docs/postgresql-v0.1/31-slice-4-implementation-notes.md`, `docs/postgresql-v0.1/32-slice-5-implementation-notes.md`, `docs/postgresql-v0.1/33-slice-6-implementation-notes.md`, `docs/postgresql-v0.1/34-slice-7-implementation-notes.md`, `docs/postgresql-v0.1/35-slice-8-implementation-notes.md`, `docs/postgresql-v0.1/36-slice-9-cli-json-contracts.md`, and `docs/postgresql-v0.1/37-slice-10-docker-automation-notes.md` for command details.

## Docker CLI Automation

Build the local CLI image:

```powershell
docker build -t dbstate-postgres:dev .
```

Run against the current repository:

```powershell
docker run --rm `
  -v "${PWD}:/workspace" `
  -w /workspace `
  dbstate-postgres:dev `
  dbstate repo status --format json
```

Use `DBSTATE_POSTGRES_URL` for session-only PostgreSQL access. Do not put credentials in the image or repository.

Docker packaging is CLI automation only. It does not add browser UI, service mode, Docker Compose, CI, SQL execution, or direct database apply.

## High-level principles

- Deterministic core first.
- Local-first security.
- Git-native workflows.
- Native RDBMS semantics.
- Cross-platform native builds.
- Browser UI as presentation layer.
- DbState Service performs real work.
- Generated SQL is reviewed, version-controlled, and human-deployed outside DbState.
- AI assists review and workflow text, but does not provide core correctness or execute deployments.

## Repository layout

- `AGENTS.md`: guidance for Codex and future agents.
- `docs/`: product-family principles and open decisions.
- `adr/`: architecture decision records.

## Contribution and workflow notes

- Work from a branch.
- Check Git status before and after changes.
- Keep product-family documents separate from edition-specific documents.
- Mark unresolved items as open decisions.
- Do not push without explicit approval.
- Do not commit secrets or local-only configuration.
