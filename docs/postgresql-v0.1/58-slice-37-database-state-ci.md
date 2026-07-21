# Slice 37: Database State CI

## Purpose

Database State CI validates the database state defined by a DbState repository branch against a disposable PostgreSQL validation database.

This is not Rust application CI. It answers:

Can the database state defined in this branch be safely reviewed, validated, and eventually promoted?

Git remains the source of truth. The repository desired-state files are the input to validation.

## Command

```powershell
dbstate ci validate `
  --repository . `
  --postgres-url "$env:DBSTATE_CI_POSTGRES_URL" `
  --disposable
```

Supported output options:

```powershell
dbstate ci validate --repository . --postgres-url "$env:DBSTATE_CI_POSTGRES_URL" --disposable --json
dbstate ci validate --repository . --postgres-url "$env:DBSTATE_CI_POSTGRES_URL" --disposable --report target\dbstate-ci\database-state-ci-report.md
```

`--disposable` is required before DbState executes any repository-defined SQL. Without it, DbState returns:

```text
Database State CI can execute repository-defined SQL only against an explicitly supplied disposable validation database. Re-run with --disposable after confirming the target database is disposable.
```

## Validation Pipeline

`dbstate ci validate` performs these steps:

1. Validate that `--disposable` is present.
2. Validate the repository path is inside a Git repository.
3. Validate the DbState project structure.
4. Reject dirty DbState-managed worktree changes.
5. Validate reference-data registry and table YAML.
6. Count release artifacts and confirm they are not executed.
7. Inspect the supplied PostgreSQL database and reject non-empty user databases.
8. Execute repository desired-state object SQL from `database/objects/` only.
9. Inspect the disposable database after build.
10. Compare the disposable database back to the repository desired state.
11. Emit text, JSON, and optional Markdown output.

## Object Build Scope

Database State CI executes only repository desired-state files under:

- `database/objects/schemas/`
- `database/objects/extensions/`
- `database/objects/enums/`
- `database/objects/domains/`
- `database/objects/sequences/`
- `database/objects/tables/` including ordinary tables and partitioned parent tables
- `database/objects/functions/`
- `database/objects/aggregates/`
- `database/objects/constraints/primary-keys/`
- `database/objects/constraints/unique-constraints/`
- `database/objects/constraints/check-constraints/`
- `database/objects/constraints/foreign-keys/`
- `database/objects/views/`
- `database/objects/materialized-views/`
- `database/objects/indexes/`
- `database/objects/triggers/`
- `database/objects/rls-policies/`
- `database/objects/grants/`

The build order is deterministic and dependency-oriented: schemas, extensions, enums, domains, sequences, tables including partitioned parent tables, functions, aggregates, constraints, views, materialized views, indexes, triggers, RLS policies, and grants.

Functions are applied with deterministic dependency retry. DbState first attempts function files in path order. If a function fails with a conservative missing-dependency error, DbState defers that function, continues applying later functions, and retries deferred functions after progress is made. Syntax errors and other non-dependency SQL errors fail immediately. If a retry pass makes no progress, CI fails and reports the remaining function file paths with safe PostgreSQL diagnostics.

Slice 37 validation against Pagila exposed missing coverage for PostgreSQL domains, partitioned parent tables, materialized-view index dependencies, and aggregate dependencies such as `public.actor_info` requiring `public.group_concat(text)`. DbState now exports domain files under `database/objects/domains/`, aggregate files under `database/objects/aggregates/`, and partitioned parent table files under `database/objects/tables/`, then applies domains before tables, functions before aggregates, aggregates before views, parent tables before constraints, and materialized views before indexes during CI rebuild.

## What Is Not Executed

Database State CI does not execute:

- `database/releases/objects/`
- `database/releases/reference-data/`
- reference-data YAML as DML
- reference-data review scripts
- generated release artifacts

Release artifacts remain review-only.

Reference-data validation in Slice 37 is YAML validation only. It does not insert, update, delete, merge, truncate, or synchronize data.

## Disposable Database Guard

The target PostgreSQL database must be user-clean before validation.

Allowed pre-existing state:

- PostgreSQL system schemas
- `information_schema`
- `pg_catalog`
- empty `public` schema

DbState refuses to run if it detects user tables, views, functions, aggregates, materialized views, sequences, enums, domains, triggers, policies, extensions, constraints, indexes, or non-bootstrap schemas. DbState does not drop, truncate, clean, reset, create, or drop databases.

## Dirty Worktree Guard

Database State CI validates committed branch state. It fails when tracked, staged, deleted, renamed, or untracked DbState-managed files are dirty under:

- `database/objects/`
- `database/reference-data/`
- `database/releases/`

DbState does not auto-clean or stash.

## JSON Output

The JSON output includes the command, success flag, repository path, validation target, project structure status, object file counts, reference-data validation, release artifact counts, compare-back result, warnings, errors, and optional report path.

Example shape:

```json
{
  "command": "ci validate",
  "success": true,
  "repositoryPath": "D:/DbState/pagila",
  "validationTarget": "disposable-postgres",
  "scope": "database-state",
  "disposable": true,
  "projectStructure": {
    "success": true,
    "missingPaths": []
  },
  "objectFiles": {
    "total": 0,
    "applied": 0,
    "skipped": 0,
    "failed": 0,
    "byType": {}
  },
  "referenceData": {
    "registryValid": true,
    "tableFiles": 0,
    "warnings": []
  },
  "releaseArtifacts": {
    "objects": 0,
    "referenceData": 0,
    "executed": false,
    "warnings": []
  },
  "comparison": {
    "similar": 0,
    "repositoryOnly": 0,
    "databaseOnly": 0,
    "different": 0,
    "unexpectedDrift": false
  },
  "warnings": [],
  "errors": [],
  "reportPath": null
}
```

## Markdown Reports

Reports are written only when `--report <path>` is supplied.

Reports are rejected under `database/` because generated CI reports are not desired-state files or release artifacts. A safe local target is:

```powershell
dbstate ci validate `
  --repository . `
  --postgres-url "$env:DBSTATE_CI_POSTGRES_URL" `
  --disposable `
  --report target\dbstate-ci\database-state-ci-report.md
```

The report includes a safety statement:

Database State CI executed repository desired-state object SQL only against the supplied disposable validation database. It did not execute release artifacts, did not execute reference-data review scripts, did not apply changes to a real environment, and did not mutate Git.

## GitHub Actions Example

This is documentation only. This repository does not add a GitHub Actions workflow in Slice 37.

```yaml
name: Database State CI

on:
  pull_request:

jobs:
  database-state:
    runs-on: ubuntu-latest
    services:
      postgres:
        image: postgres:16
        env:
          POSTGRES_PASSWORD: postgres
          POSTGRES_DB: dbstate_ci_validation
        ports:
          - 5432:5432
        options: >-
          --health-cmd pg_isready
          --health-interval 10s
          --health-timeout 5s
          --health-retries 5
    steps:
      - uses: actions/checkout@v4
      - name: Install DbState
        run: echo "Install or download the dbstate binary here"
      - name: Validate database state
        env:
          DBSTATE_CI_POSTGRES_URL: postgres://postgres:postgres@localhost:5432/dbstate_ci_validation
        run: |
          dbstate ci validate \
            --repository . \
            --postgres-url "$DBSTATE_CI_POSTGRES_URL" \
            --disposable \
            --json
```

## Safety Boundaries

Database State CI:

- does not mutate Git
- does not stage, unstage, commit, amend, switch branches, push, pull, fetch, or tag
- does not persist PostgreSQL URLs or credentials
- redacts connection credentials from output and reports
- does not execute release artifacts
- does not execute reference-data review scripts
- does not apply changes to a real environment
- does not add browser UI apply/sync behavior

## Private Beta 2 Fit

Slice 37 adds a headless validation path for branch review. The browser UI remains review-only. Human-controlled deployment remains outside DbState.
