# Product Thesis

DbState exists because database change management should be Git-native, state-based, visual, reviewable, and safe.

## Thesis

- Git is the source of truth.
- One durable database object equals one file.
- The repository stores desired database state.
- The repository is not merely a folder of historical migration scripts.
- Generated SQL is a reviewed deployment artifact, not the source of truth.
- Visual schema compare is a first-class workflow.
- Visual data compare is a first-class workflow.
- Reference data and configuration data can be versioned deliberately.
- Transactional data is not versioned by default.
- Drift between Git and a live database must be visible, explainable, and actionable.
- Dangerous database changes require explicit human review.
- DbState must be deterministic first.
- AI agents may help operate DbState, but DbState itself must compute diffs, classify risks, and generate deployment plans deterministically.
- AI must not be required for core correctness.
- Each RDBMS edition should use native SQL and native semantics.
- A universal SQL DSL must not become the core product model.

## Positioning

DbState should feel like a modern, Git-native successor to traditional schema and data compare tools.

It should combine:

- Visual compare comfort.
- Git-native state-based version control.
- Per-object schema files.
- Reference-data control.
- Drift detection.
- Safe deployment planning.
- CLI automation.
- Docker-based CI/CD usage.
- Local-first security.
- Future Codex and Claude integration through deterministic tooling.

DbState is not positioned as an AI database tool. It is a deterministic database control tool that can work well with AI agents.
