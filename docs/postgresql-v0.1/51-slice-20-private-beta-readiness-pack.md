# Slice 20 Private Beta Readiness Pack

Slice 20 prepares DbState PostgreSQL v0.1 for the first limited private beta. It is a documentation, packaging, and tester-guidance slice. It does not add product behavior.

Target tag after Slice 20 merge and post-merge smoke:

```text
v0.1.0-private-beta.1
```

Darwin creates the tag manually after merge and smoke validation.

## Safety Model

DbState remains local-first and safety-first.

DbState does not:

- execute SQL
- execute generated SQL
- apply changes to a database
- mutate PostgreSQL
- auto-stage Git changes
- auto-commit
- push, pull, fetch, or tag Git changes
- persist passwords
- persist full PostgreSQL URLs

Release artifacts remain review-only.

## Milestone Summary

### v0.1.0-alpha.1

- CLI
- Service API
- Browser UI shell
- Docker
- Workspace selection
- Safe connection profiles
- Repository to Database compare
- Database to Repository capture

### v0.1.0-alpha.2

- Expanded PostgreSQL object coverage
- Full-context Object Diff
- Release artifact hardening
- UI project initialization

### v0.1.0-alpha.3

- Windows private beta installer packaging
- installed binary smoke
- source code not distributed through installer

## Included In Private Beta

- Windows installer
- local CLI binary
- local Service API
- static browser UI
- PostgreSQL-only v0.1
- local workspace selection
- fresh Git repository initialization from UI
- non-secret connection profiles
- session URL and service environment variable connection modes
- PostgreSQL inspect
- supported PostgreSQL object coverage:
  - schemas
  - ordinary tables
  - extensions
  - enums
  - sequences
  - non-constraint-backed indexes
  - views
- Database to Repository preview
- Database to Repository write with typed confirmation and clean working tree
- Repository to Database compare
- Object Diff:
  - Full Context DDL
  - Object Only DDL
  - Related Objects
  - Raw Details
- Release artifact generation:
  - SQL review artifact
  - summary markdown
  - risk JSON
  - manifest JSON
- ParkingDemo walkthrough
- feedback template

## Not Included

- direct database apply
- SQL execution by DbState
- generated SQL execution
- PostgreSQL mutation
- automatic deployment
- Deployment Rehearsal
- hosted service mode
- multi-user mode
- authentication or user accounts
- auto-update
- license activation
- code signing
- telemetry
- MySQL, SQL Server, SQLite, Db2, or Access support
- full PostgreSQL object coverage
- procedures, aggregates, and window functions
- event triggers
- internal or constraint-generated triggers
- default privileges
- role membership grants
- column-level privileges
- policies
- roles
- ownership
- partitioning details
- row-level security
- durable comments as first-class objects
- reference-data DML generation
- arbitrary transactional data compare
- CI/CD integration
- MCP or AI integration
- Git automation beyond user-run Git commands

## Installation Options

Recommended first beta path:

1. Install with the Windows private beta installer.
2. Start `Start DbState Local Service` from the Start Menu.
3. Open `http://127.0.0.1:4587/`.
4. Follow the ParkingDemo walkthrough.

Developer path:

```powershell
cd C:\SourceCodes\DbState
cargo run -- serve --host 127.0.0.1 --port 4587
```

Docker path:

```powershell
cd C:\SourceCodes\DbState
docker build -t dbstate-postgres:dev .
```

Docker is available for testers who prefer containerized service mode, but the first Windows private beta path is the installer.

## First Tester Checklist

1. Install DbState.
2. Start DbState Local Service.
3. Open the browser UI.
4. Follow the ParkingDemo walkthrough.
5. Do not use production, UAT, staging, or shared databases.
6. Submit feedback using the template.
7. Redact secrets from screenshots, logs, and copied JSON.

## Smoke Test Matrix

Use `docs/postgresql-v0.1/54-private-beta-smoke-test-matrix.md`.

Minimum pre-beta smoke:

- installed binary help
- local service start
- health endpoint
- UI load
- ParkingDemo inspect
- Database to Repository preview
- Database to Repository write
- Repository to Database compare
- Object Diff review
- release dry-run
- release write
- uninstall
- no source files installed
- no password or full URL persistence

## Feedback Process

Feedback channel:

```text
<feedback-channel-to-be-filled-by-Darwin>
```

Use `docs/postgresql-v0.1/49-private-beta-feedback-template.md`.

Required evidence:

- screenshots where useful
- logs where useful
- reproduction steps
- DbState version or tag
- OS and browser
- native installer, Docker, or developer path

Priority levels:

- P0 blocks beta use
- P1 important beta issue
- P2 useful improvement
- P3 polish or documentation

Do not send passwords, full PostgreSQL URLs, production data, customer data, tokens, certificates, or secrets.

## Known Limitations

Use `docs/postgresql-v0.1/52-private-beta-known-limitations.md`.

High-impact limitations:

- PostgreSQL only.
- Windows installer only for the first private beta.
- Local-only service.
- No direct database apply.
- No SQL execution.
- Limited object coverage.
- Release artifacts are review-only.
- UI is beta quality.
- No code signing in the current installer package.
- No auto-update.

## Support And Escalation

Escalate immediately if a tester reports:

- DbState appears to mutate PostgreSQL.
- DbState appears to execute SQL.
- DbState writes outside the selected workspace.
- DbState persists a password or full PostgreSQL URL.
- The installer contains source files, Git history, Cargo files, tests, connection profiles, workspaces, or secrets.
- The service binds publicly without explicit user choice.

For ordinary beta feedback, ask for the completed feedback template and sanitized logs.

## Release Checklist

Before tagging private beta:

1. Confirm `dev` is clean and current.
2. Run normal cargo validation.
3. Run Docker build.
4. Build the Windows installer.
5. Install the generated setup package.
6. Smoke test installed service and UI.
7. Run the ParkingDemo golden path.
8. Verify docs use tester-facing Drive C paths.
9. Verify no secrets are included in docs, installer payload, or logs.
10. Verify known limitations are current.
11. Verify feedback template and channel placeholder are ready.
12. Verify generated installer output is not committed.

Validation commands:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo build
cargo build --release
docker build -t dbstate-postgres:dev .
```

Installer build command:

```powershell
.\packaging\windows\Build-WindowsInstaller.ps1
```

## Tag Checklist

Darwin runs these commands manually after merge and post-merge smoke:

```powershell
git switch dev
git pull origin dev
git status --short --branch --untracked-files=all
git tag -a v0.1.0-private-beta.1 -m "DbState PostgreSQL v0.1 private beta 1"
git push origin v0.1.0-private-beta.1
```

Do not create this tag from Codex.

## Rollback And Stop Criteria

Stop private beta distribution if any of these happen:

- Installed package contains source code, Git history, Cargo files, tests, local workspaces, connection profiles, or secrets.
- Service fails to start on a clean Windows tester machine.
- UI cannot load after service start.
- Health endpoint fails after service start.
- ParkingDemo inspect fails for most testers.
- Database to Repository write can write outside `database/objects/`.
- Release artifact generation emits destructive SQL.
- Raw JSON or copied output exposes passwords or full PostgreSQL URLs.
- Any workflow appears to mutate PostgreSQL or execute SQL.

Rollback action:

1. Stop distributing the installer.
2. Notify testers to uninstall.
3. Collect sanitized feedback and logs.
4. Fix on a new branch.
5. Retag only after post-fix smoke passes.
