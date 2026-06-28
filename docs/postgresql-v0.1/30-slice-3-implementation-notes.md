# Slice 3 Implementation Notes

DbState PostgreSQL v0.1 Slice 3 exports selected inspected PostgreSQL schemas and simple ordinary tables into deterministic per-object desired-state files.

This slice builds on Slice 1 repository structure recognition and Slice 2 read-only PostgreSQL catalog inspection.

No command applies SQL to a database.

## Commands

Run export commands from a Git repository that has already been initialized:

```powershell
cargo run -- init
```

Prefer the environment variable so the connection URL is not written into shell history:

```powershell
$env:DBSTATE_POSTGRES_URL = "<session-only-postgres-url>"
```

Dry-run all supported objects:

```powershell
cargo run -- export postgres --all --dry-run
```

Export all supported objects:

```powershell
cargo run -- export postgres --all
```

Export one schema scope:

```powershell
cargo run -- export postgres --schema dbstate_slice2 --dry-run
```

Export one table:

```powershell
cargo run -- export postgres --table dbstate_slice2.sample_accounts --dry-run
```

JSON output:

```powershell
cargo run -- export postgres --all --dry-run --format json
```

The command also accepts a session-only URL argument:

```powershell
cargo run -- export postgres --url "<session-only-postgres-url>" --all --dry-run
```

If both `--url` and `DBSTATE_POSTGRES_URL` are set, `--url` takes precedence.

## Credential Safety

- The connection URL is session-only.
- The URL is not persisted.
- The URL is not written to repository files.
- The URL is not included in text output, JSON output, warnings, errors, generated files, or tests.
- Persistent connection profiles are not implemented.

## Export Scope

Slice 3 exports only:

- Schemas.
- Simple ordinary/base tables.
- Columns inside exported tables.

Exported files are written under:

```text
database/objects/schemas/<schema-name>.sql
database/objects/tables/<schema-name>.<table-name>.sql
```

Generated SQL files are desired-state object definitions. They are not deployment scripts and are not generated synchronization scripts.

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

Exported table definitions are intentionally incomplete for full PostgreSQL rebuild fidelity in Slice 3.

## Repository Guards

Write export requires:

- Current directory inside a Git repository.
- Complete DbState PostgreSQL project structure.
- Clean Git working tree.
- Explicit selection through `--schema`, `--table`, or `--all`.

`dbstate export postgres --dry-run` plans writes but creates no files.

Export does not:

- Initialize missing project structure.
- Overwrite existing object files.
- Delete files.
- Stage, commit, or push Git changes.
- Create connection profiles.
- Create local machine-specific configuration.
- Create deployment artifacts under `database/releases/`.

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

- No full database-to-repo synchronization workflow exists yet.
- No source database to repository diff exists yet.
- No repository to target database compare exists yet.
- No data compare or reference-data row compare exists yet.
- No synchronization plan generation exists yet.
- No generated release script workflow exists yet.
- No browser UI exists yet.
- No Docker product runtime exists yet.
- No MCP or AI integration exists yet.
- No Deployment Rehearsal exists yet.
