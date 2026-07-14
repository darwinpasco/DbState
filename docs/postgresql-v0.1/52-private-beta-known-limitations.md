# DbState PostgreSQL v0.1.0 Private Beta 2 Known Limitations

This document lists known limitations for DbState PostgreSQL v0.1.0 Private Beta 2.

## Platform And Packaging

- PostgreSQL is the only supported database engine in this beta.
- The first beta installer is Windows-only.
- Docker is available, but the primary Windows beta path is the installer.
- The installer is not code signed in the current package.
- The installer does not modify PATH.
- The installer does not register DbState as a Windows Service.
- There is no auto-update.
- There is no license activation.

## Local Service

- The service is local-only by default.
- Do not expose the service publicly.
- There is no hosted service mode.
- There is no multi-user mode.
- There is no authentication or user account model.
- If the UI looks stale after an update, press `Ctrl+F5`.

## Safety

DbState does not:

- execute SQL
- execute generated SQL
- apply changes to a database
- mutate PostgreSQL
- generate reference-data DML
- auto-stage, commit, push, pull, fetch, or tag Git changes
- persist passwords
- persist full PostgreSQL URLs

Release artifacts are review-only and must be reviewed by a responsible engineer before any manual use outside DbState.

Private Beta 2 testers must use their own non-production PostgreSQL database. Do not use production, UAT, staging, shared, regulated, or customer-data databases. If a tester needs a sample database, Pagila is recommended because it contains tables, relationships, indexes, views, functions, and sample data:

```text
https://github.com/devrimgunduz/pagila
```

## Object Coverage

Supported beta object types:

- schemas
- ordinary tables
- extensions
- enums
- sequences
- non-constraint-backed indexes
- views
- primary key constraints
- unique constraints
- foreign key constraints
- check constraints
- regular PostgreSQL functions
- regular table and view triggers
- materialized views

Deferred object types and details:

- event triggers
- internal or constraint-generated triggers
- grants
- policies
- roles
- ownership
- partitioning details
- row-level security
- durable comments as first-class objects
- full dependency graph
- full SQL parser

Comments may appear as partial review context where available, but they are not durable first-class desired-state object types in this beta. Procedures, aggregates, and window functions remain outside the regular-function support in this beta.

## Object Diff

- Full Context DDL is a review aid, not the repository model.
- Repository storage remains normalized per durable object file.
- Object Only DDL reflects the durable object file or deterministic object renderer.
- Related Objects is intentionally narrow.
- Full Context DDL and Object Only DDL include line-by-line visual DDL comparison.
- Matched lines are white.
- Different lines are red.
- Source-only lines are green with a display-only `+` marker.
- Target-only lines are red with a display-only `-` marker.
- Target-only lines do not use strike-through.
- Plus/minus diff markers are UI indicators only. They must not be treated as source DDL, target DDL, repository object file content, release SQL, generated review artifact content, or future write/update logic.
- Unavailable related object sections should be read as `Not available in Private Beta`.
- Raw Details and Selected JSON Item are support/evidence panels, not the primary review workflow.

## Connection Handling

- Connection profiles are non-secret only.
- Passwords are session-only.
- Full PostgreSQL URLs are session-only.
- `%APPDATA%\DbState` may contain non-secret profile metadata if profiles are created.
- Do not paste production credentials, non-production credentials, customer data, regulated data, or full PostgreSQL URLs into feedback or screenshots.

## Workspace And Git

- Workspace selection is session-only.
- There is no recent projects list.
- There is no project database.
- DbState does not clone repositories.
- DbState does not fetch, pull, push, stage, commit, or tag Git changes.
- User-run Git commands remain manual.
- Repository write workflows require a clean working tree where documented.

## Browser UI

- The UI is beta quality.
- It is a static local browser UI served by `dbstate serve`.
- There is no frontend framework or Node build pipeline.
- Some error messages may still require technical interpretation.
- Object coverage and release planning are intentionally limited to the beta scope.

## Workarounds

- Use `Ctrl+F5` if the browser shows stale UI assets.
- Use the service environment variable connection mode for repeatable local tests.
- Use your own non-production PostgreSQL database for beta walkthroughs.
- Use Pagila if you need a safe sample database.
- Use `C:\DbState\PrivateBetaDemo` as the walkthrough workspace unless testing a different fresh local repository.
- Use Docker only with mounted paths that are visible inside the container, such as `/workspace`.
