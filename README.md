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
- `dbstate inspect postgres` performs read-only PostgreSQL catalog inspection for schemas, tables, columns, extensions, enums, sequences, indexes, and views.
- `dbstate export postgres` exports selected inspected schemas, simple ordinary tables, extensions, enums, sequences, indexes, and views to desired-state files under `database/objects/`.
- `dbstate sync postgres` compares selected source PostgreSQL objects against local desired-state files and creates or updates local files after explicit command invocation.
- `dbstate compare postgres` reads supported desired-state files, inspects PostgreSQL read-only, and reports repository-to-database differences without writing files.
- `dbstate plan postgres` builds an in-memory selected plan from compare results and reports limited table-to-schema dependency warnings without writing files.
- `dbstate release postgres` generates review-only SQL, summary, risk JSON, and manifest artifacts under `database/releases/` from the selected plan. It never executes the generated SQL.
- `dbstate data-compare postgres` compares explicitly configured reference-data table files against PostgreSQL rows with read-only `SELECT`.
- CLI JSON output is hardened around common `command`, `success`, `warnings`, and `errors` fields, repository context where relevant, and redaction of connection details and masked values.
- Docker packaging is available for headless CLI automation against mounted repositories.
- `dbstate serve` starts a minimal local-only HTTP JSON Service API boundary for selected read-only and plan-only operations.
- The local service serves a minimal static schema compare workflow UI shell at `/` and `/ui`, including a results grid with object-type filtering and status badges for supported PostgreSQL object types.
- Service and UI workflows can use a session-only selected local repository path through `repositoryPath`.
- Service and UI workflows can use optional local non-secret PostgreSQL connection profiles. Profiles are stored outside the repository and never store passwords, tokens, or full PostgreSQL URLs.
- Service and UI workflows can preview Database to Repository capture and, after explicit confirmation on a clean working tree, write supported desired-state object files only under the selected repository's `database/objects/` paths.
- PostgreSQL beta-minimum object coverage currently includes schemas, ordinary tables, extensions, enums, sequences, non-constraint-backed indexes, views, and configured reference-data compare.

Current implementation limitations:

- No SQL execution, direct target-database apply, reference-data DML generation, arbitrary transactional data compare, full browser UI product, database mutation workflow, Docker Compose, CI workflow, MCP, or AI integration exists yet.
- PostgreSQL access is read-only in product commands.
- No command applies SQL to a database.

See `docs/postgresql-v0.1/28-slice-1-implementation-notes.md`, `docs/postgresql-v0.1/29-slice-2-implementation-notes.md`, `docs/postgresql-v0.1/30-slice-3-implementation-notes.md`, `docs/postgresql-v0.1/31-slice-4-implementation-notes.md`, `docs/postgresql-v0.1/32-slice-5-implementation-notes.md`, `docs/postgresql-v0.1/33-slice-6-implementation-notes.md`, `docs/postgresql-v0.1/34-slice-7-implementation-notes.md`, `docs/postgresql-v0.1/35-slice-8-implementation-notes.md`, `docs/postgresql-v0.1/36-slice-9-cli-json-contracts.md`, `docs/postgresql-v0.1/37-slice-10-docker-automation-notes.md`, `docs/postgresql-v0.1/38-slice-11-service-api-boundary.md`, `docs/postgresql-v0.1/39-slice-12-browser-ui-workflow-shell.md`, `docs/postgresql-v0.1/40-slice-13-workspace-selection.md`, `docs/postgresql-v0.1/41-slice-13a-schema-compare-ui-workflow.md`, `docs/postgresql-v0.1/42-slice-13b-results-grid-usability.md`, `docs/postgresql-v0.1/43-slice-14-safe-connection-profiles.md`, `docs/postgresql-v0.1/44-slice-15-database-to-repository-workflow.md`, `docs/postgresql-v0.1/45-slice-16-postgresql-object-coverage.md`, `docs/postgresql-v0.1/46-slice-16a-object-diff-full-context-ddl.md`, `docs/postgresql-v0.1/47-slice-17-release-artifact-hardening.md`, and `docs/postgresql-v0.1/48-slice-18-golden-path-private-beta-walkthrough.md` for command details.

## Private Beta Golden Path

Technical testers can follow the repeatable ParkingDemo walkthrough in `docs/postgresql-v0.1/48-slice-18-golden-path-private-beta-walkthrough.md`.

Use `docs/postgresql-v0.1/49-private-beta-feedback-template.md` for private beta feedback. Do not include passwords, full PostgreSQL URLs, production data, or secrets in feedback.

Private beta readiness docs:

- `docs/postgresql-v0.1/51-slice-20-private-beta-readiness-pack.md`
- `docs/postgresql-v0.1/52-private-beta-known-limitations.md`
- `docs/postgresql-v0.1/53-private-beta-installer-distribution.md`
- `docs/postgresql-v0.1/54-private-beta-smoke-test-matrix.md`
- `docs/postgresql-v0.1/55-slice-21-private-beta-distribution-package.md`
- `docs/postgresql-v0.1/59-private-beta-2-release-readiness.md`
- `docs/postgresql-v0.1/60-private-beta-2-tester-onboarding.md`
- `docs/postgresql-v0.1/61-private-beta-2-tester-announcement-and-feedback.md`

## Windows Private Beta Installer

Windows private beta installer packaging is documented in `docs/postgresql-v0.1/50-slice-19-windows-installer-packaging.md`.

The packaging script is `packaging/windows/Build-WindowsInstaller.ps1`. It stages only the release binary and beta-facing installer assets, then validates that source files, Cargo files, Git history, tests, connection profiles, local workspaces, and secrets are not included.

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

For persistent non-secret Docker profiles, mount a config directory and set `DBSTATE_CONFIG_DIR`. Do not bake credentials into the image.

Docker packaging supports CLI automation and the minimal local Service API boundary. It does not add browser UI, hosted service mode, Docker Compose, CI, SQL execution, or direct database apply.

## Local Service API

Start the local Service API:

```powershell
cargo run -- serve
```

Defaults:

- Host: `127.0.0.1`
- Port: `4587`

Health check:

```powershell
Invoke-RestMethod http://127.0.0.1:4587/health
```

Slice 11 service mode exposes selected JSON endpoints for status, init planning, PostgreSQL inspect, compare, plan, and configured reference-data compare. It does not include a browser UI, authentication, user accounts, write endpoints, SQL execution, or database apply. Do not expose it publicly.

## Browser UI Shell

Start the local service and open the UI:

```powershell
cargo run -- serve
```

```text
http://127.0.0.1:4587/
```

The browser UI is a static schema compare workflow shell over the service API. It uses a left workflow navigation for workspace, source and target, compare options, results, object diff, warnings, release plan, reports, and safety information. The Results grid includes object-type filtering, status badges, a status legend, and source/target context above the table. Inspect populates schema and table dropdowns, while column details stay in Object Diff for selected tables.

Object Diff defaults to Full Context DDL for review. For selected tables, it can show the table plus related indexes where available. Object Only DDL remains available and reflects the durable normalized object file. Related Objects and Raw Details are separate tabs. Repository storage remains one durable object per file.

The UI includes Database to Repository preview and an explicit Write Repository Files action. That action requires typed confirmation, a clean working tree, and writes only supported desired-state object files under `database/objects/` in the selected repository. It does not mutate PostgreSQL, execute SQL, write release artifacts, stage Git changes, commit, push, pull, or fetch.

The Release Plan page remains review-only. Release artifact generation is performed through `dbstate release postgres`, which creates a sectioned SQL review artifact, summary markdown, risk JSON, and manifest JSON under `database/releases/`. Dry-run reports planned artifact paths and risk information without writing files.

The UI has no frontend framework, no Node build pipeline, no direct database apply, and no generated SQL execution.

The Source & Target step supports session-only URL mode, saved non-secret profile mode, and service environment variable mode. Saved profiles contain only host, port, database, username, SSL mode, and description. Passwords and full URLs remain session-only and are not persisted.

## Workspace Selection

Service and UI requests may include a session-only local repository path:

```json
{ "repositoryPath": "C:\\SourceCodes\\DbState" }
```

The path must exist, be a directory, and be inside a local Git working tree. DbState does not persist workspace paths, maintain a recent-project list, clone repositories, fetch, pull, push, stage, or commit from service endpoints. Docker users must enter a path inside the container, such as `/workspace`.

The Workspace page also includes a Browse button backed by the local Service API. It lists service-visible directories only, never files, and does not use browser filesystem APIs. In Docker, the picker can browse only paths mounted into the container.

For a selected Git repository that is not yet a DbState project, the Workspace page can initialize the standard DbState project structure after typed confirmation. Initialization writes only local folders/files such as `database/objects/`, `database/reference-data/dbstate.reference-data.yml`, and `database/releases/`. It does not capture database objects, connect to PostgreSQL, execute SQL, mutate a database, stage Git changes, commit, push, pull, or fetch.

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
