# DbState Product Family

DbState is a product family for state-based, per-object database version control, schema compare, data compare, drift detection, and safe deployment planning.

DbState is the base product strategy. It is not one generic universal database tool that hides the differences between database engines. The product should have a shared philosophy, shared architecture direction, shared UX model, and shared automation model, while allowing each RDBMS edition to use native SQL and native database semantics.

## Product family direction

Future editions may include:

- DbState PostgreSQL.
- DbState MySQL.
- DbState SQL Server.
- DbState SQLite.
- DbState Db2.
- DbState Access.

PostgreSQL is the planned first edition. This document does not define PostgreSQL v0.1 requirements.

## Why a family

Database engines differ in object models, dependency rules, permissions, DDL behavior, locking, transaction semantics, and operational risk. DbState should respect those differences instead of flattening them into a weak universal model.

Shared principles should remain consistent:

- Git is the source of truth.
- Per-object files represent desired state.
- Generated SQL is a reviewed deployment artifact.
- Visual compare is a first-class workflow.
- Drift should be visible and explainable.
- Dangerous changes require explicit human review.
- AI can assist but must not be required for correctness.

## Future repository strategy

Future RDBMS editions may remain in one repository, become packages in a monorepo, or be split or forked when the time is right. That decision is open.

No forks or separate repositories are created as part of the current foundation work.
