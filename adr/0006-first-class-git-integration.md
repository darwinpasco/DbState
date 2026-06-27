# ADR 0006: First-Class Git Integration

## Status

Accepted

## Context

DbState is Git-native. Git workflows must be part of database change management rather than external manual steps.

## Decision

DbState will provide first-class Git integration through the DbState Service, browser UI, and CLI.

Git is the source of truth for desired database state.

DbState should support clone, open repository, status, branch, checkout, fetch, pull, stage, unstage, commit, and push.

The browser UI is the presentation layer for Git workflows. The DbState Service performs Git operations.

DbState must not auto-push or auto-commit without explicit user approval. DbState must not commit secrets or credentials. Git credentials should use safe platform mechanisms.

The exact implementation approach, system Git, libgit2, or hybrid, remains an open technical decision unless decided later.

## Consequences

Git state, database object changes, generated deployment artifacts, and review workflows can be shown in one product experience.

The service must carefully protect credentials and prevent accidental commits of secrets or local-only configuration.

## Alternatives considered

- Leaving Git entirely to external tools.
- Browser UI directly invoking Git operations.

## Open questions

- System Git executable, libgit2/git2-rs, or hybrid.
- How much Git hosting integration should be built into early releases.
