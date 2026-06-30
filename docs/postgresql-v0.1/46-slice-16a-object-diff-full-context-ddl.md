# Slice 16A Object Diff Full-Context DDL

Slice 16A improves the browser Object Diff review experience without changing the DbState repository model.

Repository storage remains normalized:

- table files stay table-only
- index files stay index objects
- future constraint and comment files remain separate when implemented
- DbState does not collapse related objects into one durable table file

The UI review model is more human-friendly. Object Diff now defaults to Full Context DDL so a selected table can be reviewed with related context where available.

## Object Diff Modes

Object Diff includes four modes:

- Full Context DDL
- Object Only DDL
- Related Objects
- Raw Details

Full Context DDL is the default.

Object Only DDL shows the durable object itself. For a table, this is the normalized table object file or the deterministic database table renderer only.

Related Objects shows related summaries grouped by type. Slice 16A includes indexes where available. Constraints and comments are shown as unavailable when they are not captured yet.

Raw Details shows the selected row and object DDL response as redacted JSON.

## Full Context DDL

For selected table objects, Full Context DDL composes review text from supported sources:

- CREATE TABLE statement
- related index DDL where available
- related object summaries for indexes, constraints, and comments

For repository-side table review, DbState reads only:

- `database/objects/tables/<schema>.<table>.sql`
- matching index files under `database/objects/indexes/`

DbState does not read arbitrary files, does not read outside the selected workspace, and does not follow path traversal.

For database-side table review, DbState uses read-only PostgreSQL catalog inspection and existing deterministic renderers. It does not add new object coverage in this slice.

For non-table object types, Full Context DDL may be the same as Object Only DDL.

## Direction Rules

Object Diff keeps the Slice 15 source and target direction rules:

- Repository to Database Compare: Source is Repository, Target is Database.
- Database to Repository: Source is Database, Target is Repository.
- PostgreSQL Inspect Only: Source is Database, Target is Catalog.
- Reference Data Compare: Source is Repository, Target is Database.

Database to Repository keeps database DDL on the Source side and repository DDL on the Target side.

## Comparison State

For DDL modes, DbState normalizes leading and trailing whitespace, line endings, and repeated blank lines before comparison.

The UI shows:

- Similar when source and target DDL match
- Different when both sides exist and differ
- DDL unavailable when either side is unavailable

The label is always displayed, so the result does not depend on color alone.

## Service API

`POST /api/v1/postgres/object-ddl` remains the endpoint for object DDL detail.

The response keeps compatibility fields:

- `repositoryDdl`
- `databaseDdl`

It also returns structured sections:

- `objectOnly`
- `fullContext`
- `relatedObjects`

The UI uses these sections to switch modes without issuing another request.

## Safety Boundary

Slice 16A is display and review only.

It does not add:

- direct database apply
- SQL execution
- generated SQL execution
- PostgreSQL mutation
- destructive SQL generation
- DML generation
- Git stage, commit, push, fetch, or pull
- durable constraint object coverage
- durable comment object coverage
- frontend framework or build pipeline

## Current Limitations

- Constraint rendering in Full Context DDL is deferred.
- Table and column comments are reported as unavailable unless future slices capture them.
- Related object discovery is intentionally narrow.
- Full inline diff highlighting is not implemented.
- Object Diff full context is a review aid, not the durable repository representation.
