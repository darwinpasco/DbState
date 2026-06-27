# ADR PostgreSQL 0007: PostgreSQL v0.1 Implementation Sequencing

## Status

Proposed

## Context

DbState PostgreSQL v0.1 has a broad product surface: repository structure, Git integration, PostgreSQL inspection, object normalization, compare, dependency analysis, artifact generation, reference data, CLI, UI, Docker, and AI-review context.

Building each subsystem horizontally before proving the workflow would delay feedback and increase the risk of overbuilding.

## Decision

DbState PostgreSQL v0.1 should be implemented through incremental vertical slices rather than horizontal subsystem completion.

The first slices should prove local repository structure recognition, limited PostgreSQL inspection, selected object export, source-to-repo synchronization, repository-to-target compare, dependency-aware planning, and generated release artifacts before expanding UI, Docker, and AI-review surfaces.

## Consequences

The product can validate the source-of-truth model earlier. Each slice should produce a user-visible or test-visible outcome. Scope can be adjusted before the full UI or full PostgreSQL object set is built.

The team must keep slice boundaries tight and avoid hiding incomplete safety work behind later phases.

## Alternatives considered

- Build the full browser UI first.
- Build the full PostgreSQL adapter before any workflow.
- Build all core subsystems horizontally before a vertical demo.
- Start with Docker or CI automation.

## Open questions

- Exact Slice 1 CLI and service boundary.
- First PostgreSQL object set for Slice 2 and Slice 3.
- Whether Docker ships in v0.1 or immediately after.
