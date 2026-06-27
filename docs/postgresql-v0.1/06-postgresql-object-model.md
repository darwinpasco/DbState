# PostgreSQL Object Model for v0.1

DbState PostgreSQL v0.1 uses a native PostgreSQL object model. It does not use a universal SQL DSL as the primary product model.

## Supported Object Types

v0.1 documentation scope includes:

- Schemas.
- Extensions.
- Enums.
- Sequences.
- Tables.
- Columns.
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

## Object Identity Rules

Object identity should be stable and PostgreSQL-native.

Expected identity inputs include:

- Database object type.
- Schema name where applicable.
- Object name.
- Function argument signature where applicable.
- Constraint parent object where applicable.
- Index parent table where applicable.

Exact identity rules for overloaded functions, expression indexes, grants, and extension-owned objects are open decisions.

## Schema-Qualified File Naming

Schemas are first-class PostgreSQL objects under:

```text
database/objects/schemas/
```

Other object files should use schema-qualified file names, for example:

```text
database/objects/tables/core.payment_attempts.sql
database/objects/views/reporting.daily_sales.sql
database/objects/functions/core.calculate_balance.sql
```

Function file names may need signature-aware naming. This is an open decision.

## Normalization Rules

Normalization should make equivalent PostgreSQL definitions stable for comparison and Git review.

Expected normalization areas:

- Consistent object ordering.
- Stable formatting for generated object definitions.
- Normalized qualification of object references where needed.
- Stable column ordering from PostgreSQL metadata.
- Stable constraint and index representation.
- Function body handling that preserves meaningful body changes.
- Grant ordering.

Normalization must not hide semantic differences.

## Dependency Rules

Dependency analysis should identify selected objects that rely on excluded or missing objects.

Examples:

- View depends on table, column, function, enum, or extension.
- Function depends on table, type, enum, function, extension, or search path behavior.
- Foreign key depends on referenced table and referenced key.
- Index depends on table, columns, expressions, operators, and extensions.
- Trigger depends on trigger function.
- Materialized view depends on base objects.
- Grant depends on role and object.

Critical missing dependencies should block script generation by default.

## Partial or Deferred Object Support

Some object details may be deferred if they cannot be safely normalized in v0.1. Deferred support must be explicit in reports and open decisions.

Potential deferred areas:

- RLS policies.
- Publications and subscriptions.
- Event triggers.
- Domains.
- Collations.
- Operator classes and families.
- Full-text search objects.
- Extension-owned object management.

## Known PostgreSQL Edge Cases

- Function overloading and signature identity.
- `search_path` effects in function definitions.
- Extension-managed objects that should not be emitted as user-owned objects.
- Expression indexes and partial indexes.
- Materialized view refresh behavior.
- Grants to roles that may not exist in the target.
- Generated columns and identity columns.
- Partitioned tables.
- Object comments.

## Open Decisions

- Supported PostgreSQL versions.
- Exact object coverage for v0.1.
- Function signature file naming.
- Handling of extension-owned objects.
- Handling of partitions.
- Whether object comments are included in v0.1.
- Whether RLS policies are included or deferred.
