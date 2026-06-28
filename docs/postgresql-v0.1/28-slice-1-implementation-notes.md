# Slice 1 Implementation Notes

DbState PostgreSQL v0.1 Slice 1 implements local Git repository recognition and DbState PostgreSQL project structure initialization.

It does not implement PostgreSQL connection, schema inspection, object export, compare, data compare, SQL synchronization script generation, browser UI, Docker, MCP, AI integration, or Deployment Rehearsal.

No command applies SQL to a database.

## Commands

Run commands from a local Git repository.

```powershell
cargo run -- repo status
```

JSON output:

```powershell
cargo run -- repo status --format json
```

Dry-run initialization:

```powershell
cargo run -- init --dry-run
```

Dry-run initialization with JSON output:

```powershell
cargo run -- init --dry-run --format json
```

Initialize missing DbState PostgreSQL project structure:

```powershell
cargo run -- init
```

## Created Structure

`dbstate init` creates missing folders and the safe default reference-data registry only when explicitly invoked.

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

The default registry is:

```yaml
version: 1
tables: []
```

No reference-data tables are controlled by default.

## Safety Behavior

- `dbstate repo status` is read-only.
- `dbstate init --dry-run` reports planned creates and writes nothing.
- `dbstate init` does not overwrite existing files.
- `dbstate init` does not delete files.
- `dbstate init` is blocked when the Git working tree is dirty.
- The CLI does not create secrets, credentials, connection profiles, tokens, local machine paths, or private database details.
- The CLI has no database connection or SQL execution behavior.

## Tests

Run:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Current Limitations

- No PostgreSQL connection exists yet.
- No schema inspection exists yet.
- No object export exists yet.
- No compare workflow exists yet.
- No generated synchronization script workflow exists yet.
- No browser UI exists yet.
- No Docker image exists yet.
