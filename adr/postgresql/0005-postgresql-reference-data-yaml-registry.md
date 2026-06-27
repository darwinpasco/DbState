# ADR PostgreSQL 0005: Reference-Data Registry and Structured Files

## Status

Accepted as direction

## Context

Reference data should be versioned deliberately, while transactional data should not be versioned by default.

## Decision

DbState PostgreSQL will use an explicit reference-data registry and structured YAML or JSON table files for controlled reference data.

The registry lives at `database/reference-data/dbstate.reference-data.yml`. Table files live under `database/reference-data/tables/`.

## Consequences

Reference-data compare can be limited to configured tables, business keys, ignored columns, masked columns, compare mode, and delete behavior.

Generated reference-data DML belongs in generated synchronization scripts under `database/releases/`, not in raw SQL source files.

## Alternatives considered

- Treating all table data as version-controlled.
- Using raw SQL as primary reference-data source.
- No reference-data support in v0.1.

## Open questions

- YAML only, JSON only, or both.
- Exact registry schema.
- Exact masking and delete behavior model.
