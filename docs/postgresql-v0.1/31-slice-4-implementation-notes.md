# Slice 4 Implementation Notes

DbState PostgreSQL v0.1 Slice 4 adds source database to repository diff and approval-gated repository synchronization for supported desired-state object files.

This slice compares selected source PostgreSQL schemas and simple ordinary tables against local files under `database/objects/`. It may create added files and update changed files only when the user explicitly runs `dbstate sync postgres` without `--dry-run`.

No command applies SQL to a database.

## Commands

Run sync commands from a Git repository that has already been initialized:

```powershell
cargo run -- init
```

Prefer the environment variable so the connection URL is not written into shell history:

```powershell
$env:DBSTATE_POSTGRES_URL = "<session-only-postgres-url>"
```

Dry-run all supported objects:

```powershell
cargo run -- sync postgres --all --dry-run
```

Synchronize all supported objects:

```powershell
cargo run -- sync postgres --all
```

Synchronize one schema scope:

```powershell
cargo run -- sync postgres --schema dbstate_slice2 --dry-run
```

Synchronize one table:

```powershell
cargo run -- sync postgres --table dbstate_slice2.sample_accounts --dry-run
```

JSON output:

```powershell
cargo run -- sync postgres --all --dry-run --format json
```

The command also accepts a session-only URL argument:

```powershell
cargo run -- sync postgres --url "<session-only-postgres-url>" --all --dry-run
```

If both `--url` and `DBSTATE_POSTGRES_URL` are set, `--url` takes precedence.

## Sync Versus Export

`dbstate export postgres` is a no-overwrite export command. It creates missing desired-state files and skips existing files.

`dbstate sync postgres` is the explicit source database to repository synchronization command. It compares rendered source objects with existing local files, creates added files, updates changed files, and leaves unchanged files untouched.

Neither command stages, commits, pushes, deletes files, creates deployment artifacts, or applies SQL to a database.

## Credential Safety

- The connection URL is session-only.
- The URL is not persisted.
- The URL is not written to repository files.
- The URL is not included in text output, JSON output, warnings, errors, generated files, or tests.
- Persistent connection profiles are not implemented.

## Synchronization Scope

Slice 4 compares and synchronizes only:

- Schemas.
- Simple ordinary/base tables.
- Columns inside supported tables.

Files are written only under:

```text
database/objects/schemas/<schema-name>.sql
database/objects/tables/<schema-name>.<table-name>.sql
```

Generated SQL files are desired-state object definitions. They are not deployment scripts and are not generated synchronization scripts.

## Diff Classification

Each selected supported object is classified as:

- `added`: source object exists and no local desired-state file exists.
- `changed`: source object exists and local file content differs.
- `unchanged`: source object exists and local file content is identical.
- `skipped`: unsupported or deferred object.
- `error`: selected object does not exist, path safety fails, or file read/write fails.

When a table is selected without a local schema file, sync reports a warning. Slice 4 does not perform dependency analysis.

## Deferred Scope

The following remain deferred:

- Extensions.
- Enums.
- Sequences.
- Primary keys.
- Foreign keys.
- Unique constraints.
- Check constraints.
- Indexes.
- Views.
- Materialized views.
- Functions.
- Triggers.
- Grants.
- RLS policies.
- Comments.
- Ownership.
- Privileges.
- Reference-data rows.

Table definitions are intentionally incomplete for full PostgreSQL rebuild fidelity in Slice 4.

## Repository Guards

Write sync requires:

- Current directory inside a Git repository.
- Complete DbState PostgreSQL project structure.
- Clean Git working tree.
- Explicit selection through `--schema`, `--table`, or `--all`.

`dbstate sync postgres --dry-run` plans changes but creates or updates no files.

Sync does not:

- Initialize missing project structure.
- Delete files.
- Stage, commit, or push Git changes.
- Create connection profiles.
- Create local machine-specific configuration.
- Create deployment artifacts under `database/releases/`.
- Modify database objects or rows.

## PostgreSQL Safety

PostgreSQL access remains read-only.

Product code runs catalog inspection only. It must not execute DDL, DML, migration SQL, synchronization SQL, generated SQL, or database apply operations.

Test-only fixture setup may create schemas and tables only through the integration test harness or documented local fixture path.

## Tests

Normal tests:

```powershell
cargo test
```

Recommended validation:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional PostgreSQL integration tests use the same local fixture path introduced for Slice 2:

```text
tests/fixtures/postgresql/slice2-basic.sql
```

Run gated integration tests only against a disposable local test database:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "<disposable-test-postgres-url>"
cargo test --test postgres_integration
```

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database.

Normal `cargo test` does not require PostgreSQL. When `DBSTATE_TEST_POSTGRES_URL` is not set, integration tests return without touching a database.

## Current Limitations

- No target database compare exists yet.
- No repository to target database planning exists yet.
- No generated release script workflow exists yet.
- No data compare or reference-data row compare exists yet.
- No dependency analysis exists yet.
- No browser UI exists yet.
- No Docker product runtime exists yet.
- No MCP or AI integration exists yet.
- No Deployment Rehearsal exists yet.

## Open Decisions

- Explicit `--repo <path>` or project-folder selection should be considered later. Current behavior uses the current Git repository.
