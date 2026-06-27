# ADR 0011: AI-Assisted Git and Review Workflows

## Status

Accepted as direction

## Context

AI can help users communicate database changes, but generated text must not replace deterministic analysis or explicit user approval.

## Decision

DbState will support AI-assisted generation of Git and review workflow text.

AI may generate branch names, commit messages, PR titles, PR bodies, and comments.

AI-generated text must be grounded in deterministic DbState artifacts. AI-generated text must be editable and require user approval before use.

AI must not create branches, commit, push, create PRs, or post comments without explicit user approval.

AI must not approve database changes or override DbState risk and dependency warnings.

DbState's deterministic engine remains the authority for schema diff, data diff, risk classification, dependency analysis, and script generation.

## Consequences

DbState can reduce review writing effort while keeping deterministic artifacts and user approval as control points.

## Alternatives considered

- No AI-assisted workflow text.
- AI-generated workflow actions without explicit approval.

## Open questions

- Which hosting providers should receive first-class PR integration first.
- Exact draft-comment review flow.
