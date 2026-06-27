# PostgreSQL v0.1 Glossary

- PostgreSQL adapter: DbState component that inspects, normalizes, compares, and generates SQL for PostgreSQL-specific objects.
- PostgreSQL object identity: stable identifier for a PostgreSQL object based on object type, schema, name, and signature or parent context where needed.
- Schema-qualified object name: object name including schema, such as `core.payment_attempts`.
- PostgreSQL extension: installable PostgreSQL feature package such as `uuid-ossp` or `pgcrypto`.
- PostgreSQL enum: user-defined enumerated type.
- PostgreSQL sequence: database object that generates numeric values.
- Materialized view: PostgreSQL relation storing the result of a query that can be refreshed.
- Function body diff: comparison of PostgreSQL function implementation text and related metadata.
- Trigger function: function executed by a trigger.
- Grant drift: difference between expected and actual privileges.
- Source PostgreSQL database: database used as input for repository synchronization.
- Target PostgreSQL database: database compared against repository state for deployment planning.
- Repository synchronization: updating local repository files from a source PostgreSQL database after approval.
- Synchronization plan: selected set of changes, dependencies, warnings, and overrides used to generate artifacts.
- Synchronization artifact: generated script or report from a synchronization plan.
- Dependency warning: warning that selected and excluded objects may create an invalid or risky plan.
- Risk classification: deterministic assessment of change risk.
- Reference-data registry: `database/reference-data/dbstate.reference-data.yml`, which declares controlled reference-data tables and compare behavior.
