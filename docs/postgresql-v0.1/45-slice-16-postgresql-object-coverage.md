# Slice 16 PostgreSQL Object Coverage

Slice 16 expands DbState PostgreSQL v0.1 object coverage for private beta testing against real schemas.

The slice stays inside the existing safety model:

- PostgreSQL access is read-only.
- DbState does not execute generated SQL.
- DbState does not apply changes to a database.
- DbState does not mutate PostgreSQL.
- Repository writes are local file writes only through explicit export, sync, or Database to Repository workflows.
- Database to Repository write still requires a clean working tree and typed confirmation.

## Supported Object Types

Slice 16 supports these PostgreSQL desired-state objects:

- schemas
- ordinary tables
- extensions
- enums
- sequences
- non-constraint-backed indexes
- views

Configured reference-data compare remains separate and compare-only.

## File Naming

Supported object files are written under `database/objects/`.

```text
database/objects/schemas/<schema>.sql
database/objects/tables/<schema>.<table>.sql
database/objects/extensions/<extension>.sql
database/objects/enums/<schema>.<enum>.sql
database/objects/sequences/<schema>.<sequence>.sql
database/objects/indexes/<schema>.<table>.<index>.sql
database/objects/views/<schema>.<view>.sql
```

Invalid or ambiguous repository file names are skipped with warnings rather than guessed.

## Inspect

`dbstate inspect postgres` now reads PostgreSQL catalog state for:

- schemas
- tables
- columns
- extensions
- enums
- sequences
- indexes
- views

Inspect remains read-only and excludes internal PostgreSQL schemas such as `pg_catalog`, `information_schema`, `pg_toast*`, and other internal `pg_*` schemas where appropriate.

Inspect JSON includes arrays and counts for the supported Slice 16 object types.

## Rendering

DbState renders deterministic desired-state SQL for supported objects.

Extensions render as:

```sql
CREATE EXTENSION IF NOT EXISTS "<extension>";
```

Observed extension schema and version are recorded as comments only. Version pinning is deferred.

Enums preserve PostgreSQL enum label order:

```sql
CREATE TYPE "<schema>"."<enum>" AS ENUM (
    'label1',
    'label2'
);
```

Sequences render core sequence attributes where available:

- data type
- start value
- increment
- min value
- max value
- cache
- cycle or no cycle

Owned-by relationships are not captured in Slice 16 and are marked with a comment.

Indexes use PostgreSQL catalog definition through `pg_get_indexdef`. Constraint-backed indexes are not exported separately in Slice 16 because primary keys, unique constraints, and foreign keys remain deferred.

Views use PostgreSQL view definition through `pg_get_viewdef` and render as `CREATE VIEW`.

## Export And Sync

`dbstate export postgres` and `dbstate sync postgres` include Slice 16 object types for `--all`.

Schema scope includes schema-local object types:

- schemas
- tables
- enums
- sequences
- indexes
- views

Table scope includes the selected table and non-constraint-backed indexes on that table.

Extensions are database-level objects and are included under `--all`.

No `--type` filter is added in Slice 16. Fine-grained object-type selection remains deferred.

Export and sync write only local files under `database/objects/`. They do not mutate PostgreSQL and do not execute SQL.

## Compare

`dbstate compare postgres` compares repository desired-state files against read-only PostgreSQL catalog renderings for supported Slice 16 object types.

Classifications remain:

- `inSync`
- `repoDifferent`
- `repoOnly`
- `databaseOnly`
- `skipped`

Comparison normalizes line endings and trailing whitespace. Slice 16 does not implement a full SQL parser, so meaningful SQL text differences still require review.

## Plan And Release

`dbstate plan postgres` includes supported Slice 16 object types in the same object-level plan model used by earlier slices.

Dependency warnings remain intentionally limited. Slice 16 checks simple schema dependencies and keeps full dependency graph extraction deferred.

`dbstate release postgres` can include reviewable CREATE text for repo-only supported objects. For changed objects, release artifacts emit review comments. Slice 17 standardizes the current review comment as:

```sql
-- REVIEW REQUIRED: object differs; automatic ALTER is not generated in Slice 17.
```

Release generation never executes SQL.

Release generation does not generate destructive SQL such as `DROP`, `TRUNCATE`, `DELETE`, destructive `ALTER`, `REVOKE`, or privilege-removal SQL. Later grant support generates review-only `GRANT` SQL for repository-only explicit object grants.

Slice 17 hardens release bundles with sectioned SQL, summary markdown, risk JSON, and manifest JSON under `database/releases/`.

## Service And UI

The local Service API includes the expanded object types through existing endpoints:

- `POST /api/v1/postgres/inspect`
- `POST /api/v1/postgres/compare`
- `POST /api/v1/postgres/plan`
- `POST /api/v1/postgres/repository-sync/preview`
- `POST /api/v1/postgres/repository-sync/write`
- `POST /api/v1/postgres/object-ddl`

The browser UI Results grid can show:

- Schema
- Table
- Extension
- Enum
- Sequence
- Index
- View
- Reference data, only in reference-data workflow
- Unknown or skipped, only when present

Object Diff requests source and target DDL for supported object types. If DDL is unavailable, the UI shows a clean unavailable state instead of inventing SQL.

Slice 16A expands the Object Diff review surface. Full Context DDL is the default view and can compose table DDL with related index DDL where available. Object Only DDL remains available and reflects the normalized durable object file. Related Objects and Raw Details remain separate views. This does not add durable constraint or comment coverage and does not change repository storage.

## Optional Integration Fixture

`tests/fixtures/postgresql/slice16-object-coverage.sql` contains a disposable PostgreSQL fixture with an extension, enum, sequence, table, index, and view.

Gated PostgreSQL tests still require:

```powershell
$env:DBSTATE_TEST_POSTGRES_URL = "postgres://postgres:dbstate_test_only@localhost:<PORT>/dbstate_slice16_test"
cargo test --test postgres_integration
```

Do not point `DBSTATE_TEST_POSTGRES_URL` at production, UAT, staging, or any shared database unless read-only inspection has been explicitly approved.

## Current Deferrals

The following remain deferred:

- procedures, aggregates, and window functions
- event triggers
- internal or constraint-generated triggers
- default privileges
- role membership grants
- column-level privileges
- roles
- ownership
- partitioning details
- table-level RLS enable/force state changes
- full dependency graph
- full SQL parser
- object-type-specific CLI `--type`

These are deferred to avoid adding unsafe or under-specified behavior in the beta-minimum object coverage slice.
