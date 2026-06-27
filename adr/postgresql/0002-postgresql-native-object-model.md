# ADR PostgreSQL 0002: Native PostgreSQL Object Model

## Status

Accepted

## Context

PostgreSQL has native object types, dependency behavior, extension behavior, function signatures, grants, and DDL semantics that should not be flattened into a weak universal model.

## Decision

DbState PostgreSQL will use a native PostgreSQL object model instead of a generic universal SQL model.

## Consequences

The PostgreSQL adapter must handle PostgreSQL-specific inspection, normalization, dependency analysis, SQL generation, and risk classification.

Object identity and file naming must account for PostgreSQL details such as schemas, overloaded functions, extension-owned objects, indexes, constraints, and grants.

## Alternatives considered

- Universal SQL DSL as the primary model.
- Generic RDBMS object model with PostgreSQL extensions added later.

## Open questions

- Exact supported object set for v0.1.
- Handling of function overloads, partitions, RLS policies, and extension-owned objects.
