# RDBMS Edition Roadmap

DbState should start with one edition and grow into a family of native RDBMS editions.

## Planned first edition

DbState PostgreSQL is the planned first implementation target.

This roadmap stays general. It does not define PostgreSQL v0.1 BRD, PRD, SDD, feature scope, or detailed implementation design.

## Future editions

- DbState SQL Server.
- DbState MySQL.
- DbState SQLite.
- DbState Db2.
- DbState Access.

## Shared expectations

Each edition should share:

- State-based, per-object version control.
- Git as the source of truth.
- Browser UI presentation model.
- DbState Service orchestration.
- Deterministic core engine behavior.
- CLI automation.
- Docker automation image.
- Version-controlled deployment artifacts.
- No direct apply to real target databases.
- Dependency-aware selective synchronization.

Each edition should differ where the database engine differs.
