# PostgreSQL v0.1 Implementation Open Decisions

## Decisions Inherited from Product Docs

- Supported PostgreSQL versions.
- Exact Rust web framework.
- Exact UI framework.
- Exact local service port strategy.
- Exact credential store strategy.
- Exact Git implementation approach.
- Exact release artifact naming convention.
- YAML versus JSON reference-data files.
- Exact CLI command names and option names.
- Exact CLI JSON schemas.
- Whether MCP is MVP or post-MVP.
- Which PostgreSQL edge cases are deferred.
- Whether Docker image ships in v0.1 or immediately after.
- Packaging strategy for Windows, macOS, and Linux.
- Exact risk severity taxonomy.
- Exact dependency override policy.

## New Implementation Decisions Discovered During Planning

- Whether Slice 1 exposes only CLI behavior first or both CLI and service behavior.
- Whether project initialization creates an empty reference-data registry by default.
- Whether validation treats missing `database/releases/` as warning or error.
- Whether Git status should be required for all project operations.
- Whether source and target connection profiles are session-only for early slices.
- Whether generated artifact naming can use a temporary convention before final release naming.
- Whether golden files are required before each object type enters MVP.
- Whether UI implementation waits until CLI/service contracts stabilize.

## Recommended Priority Order

1. Git implementation approach.
2. Slice 1 CLI/service boundary.
3. Project initialization behavior.
4. Reference-data registry default behavior.
5. PostgreSQL version support.
6. Catalog inspection strategy.
7. Object identity and file naming for first object set.
8. Credential store strategy.
9. CLI JSON schema approach.
10. Release artifact naming convention.
11. Dependency warning severity model.
12. Docker ship timing.
13. MCP timing.
14. Platform packaging formats.

## Decisions Blocking Slice 1

- Slice 1 CLI/service boundary.
- Project initialization behavior.
- Whether missing folders are warnings or errors.
- Whether an empty reference-data registry is created during initialization.
- Basic Git status implementation approach.

## Decisions That Can Wait

- Full PostgreSQL object coverage.
- Function signature file naming beyond the first supported function slice.
- Extension-owned object policy.
- Partitioned table support.
- Object comment support.
- RLS policy support.
- Docker ship timing.
- MCP timing.
- Native installer formats.
- Deployment Rehearsal naming and tier placement.

## Non-Negotiable Boundaries

- Git is the source of truth.
- Per-object files are desired state.
- Generated SQL is a version-controlled deployment artifact.
- DbState must not directly apply generated SQL to target PostgreSQL databases.
- No direct target-database apply is exposed through browser UI, service API, CLI, Docker, MCP, or AI integration.
- Deployment Rehearsal is premium later, not MVP.
- Non-PostgreSQL engines stay out of v0.1 implementation scope.
