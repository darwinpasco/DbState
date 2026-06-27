# ADR 0002: Product Family with Native RDBMS Editions

## Status

Accepted

## Context

Database engines differ in SQL dialects, object models, dependency rules, deployment behavior, permissions, and operational risk.

## Decision

DbState will be a product family with native RDBMS editions, not a lowest-common-denominator universal database tool.

## Consequences

Each edition can use native SQL and native semantics. Shared product philosophy, UX, architecture direction, CLI behavior, and repository format should remain consistent where practical.

Edition-specific work will require adapter-level design and testing.

## Alternatives considered

- One universal abstraction over all supported databases.
- Separate unrelated products per database engine.

## Open questions

- Future repository strategy for editions.
- Exact boundaries between shared core and native adapters.
