# ADR 0001: State-Based Per-Object Version Control

## Status

Accepted

## Context

DbState needs a clear source-of-truth model for database state. Migration-only workflows preserve history but can make the desired end state harder to inspect, compare, and reason about.

## Decision

DbState will use state-based, per-object database version control as the core product philosophy.

Git is the source of truth. One durable database object equals one file. Generated SQL is a reviewed deployment artifact, not the source of truth.

## Consequences

Users can review desired database state in Git. Visual compare can map live database objects to repository files. Deployment scripts can be regenerated from deterministic diff and planning logic.

DbState must invest in deterministic normalization, dependency analysis, and native RDBMS adapters.

## Alternatives considered

- Migration-only repository model.
- A mixed model where migration scripts are the primary source of truth.
- A universal SQL DSL as the primary source model.

## Open questions

- Exact per-object file naming conventions by RDBMS.
- How much generated SQL history should be retained by default.
