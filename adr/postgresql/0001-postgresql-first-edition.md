# ADR PostgreSQL 0001: PostgreSQL First Edition

## Status

Accepted

## Context

The general DbState foundation identifies PostgreSQL as the planned first RDBMS-specific edition.

## Decision

DbState PostgreSQL will be the first RDBMS-specific edition.

## Consequences

PostgreSQL v0.1 will be used to prove the DbState thesis: Git as source of truth, per-object desired-state files, visual compare, repository synchronization, script generation, version-controlled deployment artifacts, and no direct target-database apply.

Future editions remain out of scope for v0.1 implementation planning.

## Alternatives considered

- Starting with a different RDBMS.
- Building a generic universal SQL product first.

## Open questions

- Supported PostgreSQL versions.
- Exact PostgreSQL v0.1 packaging and release scope.
