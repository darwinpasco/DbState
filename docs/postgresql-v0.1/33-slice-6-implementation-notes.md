# DbState PostgreSQL v0.1 Slice 6 Implementation Notes

Slice 6 adds an in-memory selected synchronization plan model with limited dependency warnings.

The command is read-only:

- It reads supported desired-state files under `database/objects/schemas/` and `database/objects/tables/`.
- It inspects PostgreSQL schemas, ordinary tables, and columns through read-only catalog queries.
- It reuses Slice 5 compare classifications.
- It builds selected plan metadata for future script generation.
- It does not write repository files.
- It does not create files under `database/releases/`.
- It does not generate SQL synchronization scripts.
- It does not execute DDL, DML, synchronization SQL, generated SQL, deployment SQL, or database apply operations.

## Command

Use an environment variable for the session-only PostgreSQL URL where practical:

```powershell
$env:DBSTATE_POSTGRES_URL = "<session-only-postgres-url>"
```

Plan all supported actionable differences:

```powershell
cargo run -- plan postgres --all
```

Plan one schema scope:

```powershell
cargo run -- plan postgres --schema dbstate_slice2
```

Plan one supported table:

```powershell
cargo run -- plan postgres --table dbstate_slice2.sample_accounts
```

Return JSON:

```powershell
cargo run -- plan postgres --all --format json
```

The command also accepts `--url <session-only-postgres-url>`, but environment variable usage is preferred because command-line arguments can be retained in shell history.

## Cherry-Pick Selection

Object references use these formats:

```text
schema:<schema-name>
table:<schema-name>.<table-name>
```

Include one actionable object:

```powershell
cargo run -- plan postgres --all --include table:dbstate_slice2.sample_accounts
```

Exclude one object:

```powershell
cargo run -- plan postgres --all --exclude schema:dbstate_slice2
```

If `--include` is not provided, all actionable differences in the selected scope are selected by default unless excluded.

Dependency auto-inclusion is not implemented in Slice 6. DbState reports required dependency warnings instead of silently adding objects.

## Plan Versus Compare, Export, And Sync

`dbstate export postgres` writes missing desired-state files and skips existing files.

`dbstate sync postgres` is the explicit source database to repository synchronization command. It can create added files and update changed files under `database/objects/`.

`dbstate compare postgres` is read-only and reports repository-to-database differences.

`dbstate plan postgres` is also read-only. It converts actionable compare differences into selected plan metadata and dependency warnings. It does not generate deployment SQL and does not create deployment artifacts.

## Supported Scope

Slice 6 plans only:

- Schemas.
- Simple ordinary PostgreSQL tables.
- Columns inside supported tables, as part of the rendered table definition.

Deferred object types are reported as deferred and are not planned deeply yet:

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

Table definitions remain intentionally incomplete for full rebuild fidelity in Slice 6.

## Plan Intents

Slice 6 emits intent labels only. They are not executable steps:

- `createInDatabaseLater`: repository-only desired-state object may need to be created in a future target database script.
- `updateDatabaseLater`: repository and database differ, so a future target database script may need an update.
- `reviewDatabaseOnly`: database-only object needs human review before deciding whether it should be added to the repository or handled later.
- `blocked`: selected object has a critical missing dependency.

In-sync objects do not produce actionable plan items.

## Dependency Warnings

Slice 6 implements one dependency rule:

- A table desired-state object depends on its schema desired-state object.

Warnings currently include:

- `missingDependency`: selected table requires a schema desired-state file that is missing.
- `excludedRequiredDependency`: selected table depends on an explicitly excluded schema.
- `dependentObjectImpacted`: excluded schema impacts selected table objects.

A selected table with a missing or excluded schema dependency is blocked. Blocked items are not ready for future script generation.

Foreign keys, views, functions, triggers, grants, materialized views, and reference-data dependencies are deferred.

## Safety

The plan command:

- Does not persist the PostgreSQL URL.
- Does not write the PostgreSQL URL to repository files.
- Redacts connection strings from text output, JSON output, warnings, and errors.
- Does not create connection profiles.
- Does not create local machine-specific configuration.
- Does not stage, commit, or push Git changes.
- Does not read or write `database/releases/`.
- Does not apply SQL to a database.

The command may run when the working tree is dirty because it is read-only, but it reports repository status where available.

## Tests

Run normal tests:

```powershell
cargo test
```

Run formatting and lint checks:

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo build
```

Optional PostgreSQL integration tests use the existing local fixture path:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "<local-disposable-test-postgres-url>"
cargo test --test postgres_integration
```

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database. The fixture SQL is test-only and may create schemas and tables in the configured disposable database.

## Current Limitations

- No SQL synchronization script is generated.
- No deployment artifacts under `database/releases/` are generated.
- No target database apply exists.
- No full dependency graph exists yet.
- No foreign key, view, function, trigger, grant, materialized view, or reference-data dependency analysis exists yet.
- No data compare or reference-data row compare exists yet.
- No Deployment Rehearsal exists yet. It remains a premium later feature direction, not Slice 6 scope.
- No browser UI, DbState Service API, Docker product runtime, MCP server, or AI integration exists yet.

## Open Decision

Explicit `--repo <path>` or project-folder selection should be considered later. Current Slice 6 behavior uses the current Git repository.
