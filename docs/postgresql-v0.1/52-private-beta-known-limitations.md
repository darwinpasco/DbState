# Private Beta Known Limitations

This document lists known limitations for the first DbState PostgreSQL v0.1 private beta.

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

## Object Coverage

Supported beta object types:

- schemas
- ordinary tables
- extensions
- enums
- sequences
- non-constraint-backed indexes
- views

Deferred object types and details:

- functions
- triggers
- grants
- policies
- roles
- ownership
- materialized views
- partitioning details
- row-level security
- durable constraints as first-class objects
- durable comments as first-class objects
- full dependency graph
- full SQL parser

Constraints and comments may appear as partial review context where available, but they are not durable first-class desired-state object types in this beta.

## Object Diff

- Full Context DDL is a review aid, not the repository model.
- Repository storage remains normalized per durable object file.
- Object Only DDL reflects the durable object file or deterministic object renderer.
- Related Objects is intentionally narrow.
- Inline diff highlighting is not implemented.

## Connection Handling

- Connection profiles are non-secret only.
- Passwords are session-only.
- Full PostgreSQL URLs are session-only.
- `%APPDATA%\DbState` may contain non-secret profile metadata if profiles are created.
- Do not paste production credentials into feedback or screenshots.

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
- Use disposable local databases for beta walkthroughs.
- Use `C:\DbState\ParkingDemo` for the ParkingDemo workspace unless testing a different local repository.
- Use Docker only with mounted paths that are visible inside the container, such as `/workspace`.
