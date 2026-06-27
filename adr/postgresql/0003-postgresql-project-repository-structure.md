# ADR PostgreSQL 0003: PostgreSQL Project Repository Structure

## Status

Accepted

## Context

DbState needs a predictable repository layout for desired-state objects, controlled reference data, and generated deployment artifacts.

## Decision

DbState PostgreSQL projects will use the `database/objects`, `database/reference-data`, and `database/releases` structure.

PostgreSQL schemas are first-class objects under `database/objects/schemas/`. Other object files should use schema-qualified names such as `core.payment_attempts.sql`.

## Consequences

Per-object files remain the desired-state source of truth. Configured reference data is explicit. Generated SQL scripts and review artifacts are version-controlled under a release artifact folder.

The exact artifact naming convention remains open.

## Alternatives considered

- Using `database/schema/` or `database/schemas/` as the root.
- Treating generated scripts as temporary local files.
- Storing reference data as raw SQL source files.

## Open questions

- Exact file naming for overloaded functions.
- Exact release artifact naming convention.
- Whether YAML, JSON, or both are used for reference data.
