# First Implementation Slice

## Goal

Build DbState PostgreSQL project structure recognition and local repository initialization.

This is the first actual coding task after planning. This document does not write code.

## User Story

As a DbState PostgreSQL user, I want to open a local Git repository and know whether it has the expected DbState project structure so I can initialize it before connecting to a PostgreSQL database.

## Scope

- Detect Git repository root.
- Read current branch and Git status.
- Detect `database/objects/`.
- Detect `database/reference-data/`.
- Detect `database/reference-data/dbstate.reference-data.yml`.
- Detect `database/reference-data/tables/`.
- Detect `database/releases/`.
- Report missing structure.
- Initialize missing structure only after explicit approval.
- Preserve no-secret and no-local-only configuration boundaries.

## Non-Scope

- PostgreSQL connection.
- Schema inspection.
- Object export.
- Schema compare.
- Script generation.
- Reference-data row compare.
- Browser UI polish.
- Docker image.
- MCP integration.
- Direct target-database apply.

## Conceptual Files or Modules Likely Needed

The future implementation will likely need conceptual areas for:

- Project discovery.
- Repository layout validation.
- Initialization plan.
- Git status adapter.
- CLI command handling.
- Service project API.
- Tests and fixtures.

This planning task does not create those files or modules.

## CLI Behavior Conceptually

Potential commands:

- `dbstate init`
- `dbstate repo status`

Expected behavior:

- Report whether the current folder is a Git repository.
- Report whether DbState PostgreSQL structure exists.
- Show missing folders and files.
- Support JSON output.
- Require explicit approval before creating structure.

Exact command names are open decisions.

## Service Behavior Conceptually

Potential service areas:

- Open project.
- Validate project.
- Initialize project structure.
- Show Git status.

The service performs filesystem and Git work. The browser UI only presents results and asks for confirmation.

## Tests Required

- Valid Git repository with no DbState structure.
- Valid Git repository with complete structure.
- Missing `database/releases/`.
- Missing reference-data registry.
- Non-Git folder.
- Dirty working tree warning.
- Initialization is approval-gated.
- Initialization does not create secrets, connection profiles, or local-only machine paths.
- JSON output contract test.

## Acceptance Criteria

- A local repository can be recognized.
- Current Git branch and status are reported.
- Missing DbState folders and registry are reported.
- Initialization behavior is approval-gated.
- Generated project structure matches the documented layout.
- No PostgreSQL connection is required.
- No implementation path applies SQL or creates database scripts.
- No secrets or local-only configuration are created.

## Follow-Up Slice

Slice 2 should add PostgreSQL connection profile planning and schema inspection for a limited object set.
