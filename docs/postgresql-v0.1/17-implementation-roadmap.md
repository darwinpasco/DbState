# PostgreSQL v0.1 Implementation Roadmap

## Purpose

This roadmap translates the DbState foundation and PostgreSQL v0.1 product documents into an incremental engineering plan.

The implementation should prove the end-to-end workflow through small vertical slices. It should not try to build the full product surface before validating the core path.

## Scope

The roadmap covers planning for:

- Local repository structure recognition.
- PostgreSQL schema inspection.
- Per-object file export and import.
- Source database to repository synchronization.
- Repository to target database comparison.
- Dependency-aware synchronization planning.
- Version-controlled SQL artifact generation.
- Basic configured reference-data compare.
- CLI JSON output.
- Browser UI workflow integration.
- Docker automation.
- AI-review context generation.

## Assumptions

- PostgreSQL is the only implementation target for v0.1.
- Git is the source of truth.
- Per-object files are the desired-state source of truth.
- Generated SQL synchronization scripts are version-controlled deployment artifacts.
- DbState Service performs work. Browser UI presents workflows.
- DbState must not directly apply generated SQL to target PostgreSQL databases through UI, service API, CLI, Docker, MCP, or AI integration.
- Deployment Rehearsal is premium later, not MVP.
- Preferred technology direction is Rust core, Rust service, React plus TypeScript UI, Rust CLI, and Docker automation, but final choices remain open decisions.

## Roadmap Phases

### Phase 0: Planning and Technical Spikes

Proves that the team has enough technical evidence to start the smallest implementation slice.

Dependencies:

- Product docs and ADRs merged.
- Open decisions reviewed.

Exit criteria:

- Slice 1 scope is agreed.
- Required spikes for project structure and repository handling are identified.
- No implementation scaffolding is created by planning work.

### Phase 1: Local Project Foundation

Proves that DbState can recognize and initialize a PostgreSQL project repository layout.

Dependencies:

- Repository structure ADR.
- Git status behavior decision for local workflows.

Exit criteria:

- Project structure detection is defined.
- Initialization behavior is specified.
- Dirty working tree and Git status behavior are understood.

### Phase 2: Limited PostgreSQL Inspection to Files

Proves that DbState can connect to PostgreSQL, inspect a limited object set, normalize it, and export selected objects to per-object files.

Dependencies:

- PostgreSQL catalog inspection spike.
- Object identity and naming decisions for the first object set.

Exit criteria:

- Schemas and a limited table object path are proven.
- Golden-file expectations are defined.
- Unsupported object reporting is defined.

### Phase 3: Database to Repository Synchronization

Proves source database to repository synchronization with explicit user approval.

Dependencies:

- File export/import.
- Git status integration.
- Basic compare model.

Exit criteria:

- Selected source objects can update local desired-state files.
- Generated changes are visible in Git status.
- Stage, commit, and push remain approval-gated.

### Phase 4: Repository to Target Planning

Proves repository state can be compared to a target PostgreSQL database without applying changes.

Dependencies:

- Repository file parsing.
- Target inspection.
- Object diff model.

Exit criteria:

- Repo-to-target differences are reported.
- No target mutation exists in the design or tests.
- Planning output is deterministic.

### Phase 5: Dependency-Aware Script Artifacts

Proves cherry-picking, dependency warnings, risk classification, and generated release artifacts.

Dependencies:

- Dependency graph spike.
- SQL generation approach.
- Artifact naming decision or temporary convention.

Exit criteria:

- Critical missing dependencies block script generation by default.
- Non-critical overrides are recorded.
- SQL, summary, risk, and AI-review context artifacts are generated into the repository.

### Phase 6: Reference Data, CLI, UI, Docker, and AI Review

Proves the remaining MVP surface around the core path.

Dependencies:

- Reference-data format decision.
- CLI command and JSON contract decisions.
- UI workflow integration.
- Docker automation shape.
- AI-review context format.

Exit criteria:

- Configured reference-data compare works for MVP scenarios.
- CLI emits JSON for key workflows.
- UI can drive the core workflow through the service.
- Docker supports headless automation.
- AI-review context contains deterministic, non-secret artifacts.

## Non-Goals

- Non-PostgreSQL implementation scope.
- Direct target-database apply.
- Deployment Rehearsal in MVP.
- Cloud dashboard.
- Self-hosted team server.
- RBAC, SSO, policy-as-code, and compliance packs.
- Full PostgreSQL object coverage before the core workflow is proven.
