# PostgreSQL v0.1 CLI and Service API

This document defines the initial product and design-level surface. It does not implement commands or API routes.

## CLI Principles

- Important commands should support JSON output.
- Commands should be local-first.
- Commands should not expose raw credentials.
- Commands may inspect, compare, plan, report, and generate artifacts.
- Commands must not directly apply generated SQL to target PostgreSQL databases.

## Proposed CLI Commands

### `dbstate init`

Initializes DbState project structure in a Git repository.

JSON output should report created or existing folders and warnings.

### `dbstate repo clone`

Clones a remote Git repository through DbState workflow.

JSON output should report repository path, branch, and status.

### `dbstate repo status`

Shows Git status and DbState project status.

JSON output should include branch, dirty state, changed object files, generated artifacts, and conflicts.

### `dbstate inspect`

Inspects a PostgreSQL database and emits normalized inventory.

JSON output should include object list, dependency data where available, warnings, and unsupported objects.

### `dbstate export`

Exports or synchronizes selected PostgreSQL objects from a source database into repository files after approval.

JSON output should include written files, skipped files, warnings, and Git status.

### `dbstate compare`

Compares source database to repository or repository to target database.

JSON output should include object diffs, data diffs where configured, and summary counts.

### `dbstate plan`

Creates a selected synchronization plan.

JSON output should include selected objects, excluded objects, required dependencies, missing dependencies, warnings, and blockers.

### `dbstate risk`

Classifies risk for a selected plan.

JSON output should include risk categories, destructive changes, dependency issues, and override requirements.

### `dbstate data-compare`

Compares configured reference-data tables.

JSON output should include inserts, updates, deletes, masked values, ignored columns, and delete behavior warnings.

## Service API Areas

The DbState Service should expose local API areas for:

- Project management.
- Git operations.
- Connection profiles.
- Schema inspection.
- Schema compare.
- Reference-data compare.
- Synchronization plan generation.
- Artifact generation.
- Risk reporting.
- AI-review context generation.

Exact API routes are open decisions.

Example route shapes may include:

- `POST /projects/open`
- `POST /git/status`
- `POST /postgres/inspect`
- `POST /compare/schema`
- `POST /compare/reference-data`
- `POST /plans`
- `POST /artifacts`

These examples are not final route decisions.

## Open Decisions

- Exact command names and option names.
- JSON schema format.
- Exit code conventions.
- Local API route design.
- Authentication model for local service access.
- Whether service API is documented publicly in v0.1.
