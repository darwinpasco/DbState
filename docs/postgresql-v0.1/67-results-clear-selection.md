# Results Clear Selection

This note documents the repository-sync Results **Clear Selection** action.

The action removes all objects from the current repository-write selection. It does not delete Results, delete repository files, mutate PostgreSQL, execute SQL, run Git, or re-run Preview Repository Sync.

## Purpose

Database-to-repository previews can produce hundreds of planned repository object rows. The default preview selection is useful for broad capture, but targeted workflows need a first-class way to clear that selection before choosing a small set of repository object files.

For Video 1, this allows the initial schema capture to select only:

```text
database/objects/tables/public.country.sql
database/objects/tables/public.actor.sql
```

The runtime implementation remains generic and does not hard-code those demo paths.

## Source Of Truth

The browser Results state maintains an authoritative in-memory include set. Include checkboxes read from that set, and repository-sync write sends the selected unique repository-relative paths to the service.

For planned repository rows, the unique selected identity is the normalized repository-relative object path, for example:

```text
database/objects/tables/public.country.sql
database/objects/aggregates/public.group_concat.text.sql
database/objects/constraints/primary-keys/public.actor.actor_pkey.sql
```

The selected count is based on unique repository paths, not rendered rows. Duplicate Results representations of the same path count once.

## Selectors

- `results-clear-selection`: visible semantic button labeled `Clear Selection`
- `results-selected-count`: machine-readable integer count of unique selected repository paths
- `results-selection-status`: machine-readable state value
- `results-selection-clearing`: active while clearing is being processed
- `results-selection-cleared`: durable after the selected path count is `0`
- `results-selection-error`: controlled clearing error text

Stable status values:

```text
idle
clearing
cleared
error
```

`results-selected-count` contains only an integer:

```text
0
2
332
```

## Behavior

When the user clicks **Clear Selection**, DbState:

1. Requires Results to be loaded.
2. Clears the authoritative include set in one product operation.
3. Re-renders Results as needed.
4. Reconciles duplicate row representations through the same include set.
5. Sets `results-selected-count` to `0`.
6. Disables repository write while no repository object paths are selected.
7. Preserves the loaded comparison and preview data.

After clearing, the user can select individual planned rows normally. The repository write request includes only the selected repository-relative paths.

## Repository Write

Repository-sync write rejects an explicit empty include selection. When paths are selected, the sync write plan is filtered to those exact repository-relative object paths.

Selected paths must:

- be repository-relative
- use safe path segments
- be under `database/objects/`
- match the current planned repository write set
- be unique

Backslashes are normalized to forward slashes for comparison. Absolute paths, traversal paths, duplicate paths, paths outside `database/objects/`, and paths not in the current planned write set are rejected.

## Filters And Duplicate Rows

Clear Selection clears the full authoritative selection, not just the currently visible filter subset. Hidden selected rows and duplicate row representations are reconciled through the same include set.

The selected count represents unique selected repository paths. It is not the number of rendered rows, checked DOM controls, or visible filtered rows.

## Safety Boundaries

Clear Selection does not:

- inspect PostgreSQL again
- modify PostgreSQL
- execute SQL
- write repository files
- stage, commit, push, branch, switch, fetch, or pull Git state
- change protected-branch enforcement
- weaken typed repository-write confirmation
- persist or expose credentials

DbState still requires explicit repository-write confirmation and Git guardrails before writing selected repository object files.
