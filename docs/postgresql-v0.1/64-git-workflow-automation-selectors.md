# Git Workflow Automation Selectors

This note documents stable browser automation selectors for DbState `Git Workflow` recommendations.

DbState remains the authority for suggested Git workflow values shown in the browser. A future external Git client automation may read these values and perform Git operations outside DbState, but DbState does not create branches, switch branches, stage files, commit, push, pull, fetch, or tag.

## Recommendation Contract

Machine-readable recommendation values are separate from presentation text.

Automation should read exact values from these elements:

- suggested branch name: `git-workflow-suggested-branch-name`
- suggested files: `git-workflow-suggested-file-path-{repository-path-slug}`
- suggested commit title: `git-workflow-suggested-commit-title`
- suggested commit body: `git-workflow-suggested-commit-body`

The value elements contain only the value to use. They do not include labels, shell commands, bullets, quotation marks, or explanatory text.

## Panel and State Selectors

- `git-workflow-panel`
- `git-workflow-status`
- `git-workflow-ready-state`
- `git-workflow-blocked-state`
- `git-workflow-no-changes-state`
- `git-workflow-loading-state`
- `git-workflow-error-state`

Only the applicable state is visible. The status container also exposes `data-state` with one of:

- `ready`
- `blocked`
- `no-changes`
- `loading`
- `error`

## Suggested Branch

- `git-workflow-suggested-branch`
- `git-workflow-suggested-branch-name`
- `git-workflow-copy-branch-name`
- `git-workflow-branch-reason`

`git-workflow-suggested-branch-name` contains only the branch name, for example:

```text
dbstate/schema-export/20260724-113631-e0f39c-all
```

It does not contain `git switch -c`, explanatory text, or quotes.

## Suggested Files

- `git-workflow-suggested-files`
- `git-workflow-suggested-files-count`
- `git-workflow-suggested-files-empty`
- `git-workflow-suggested-file-{repository-path-slug}`
- `git-workflow-suggested-file-path-{repository-path-slug}`

Each path element contains only the exact repository-relative path.

Example:

```text
database/objects/tables/public.country.sql
```

The suggested-file selector uses the same repository-path slug as Repository Files:

```text
repository-file-row-database-objects-tables-public-country-sql-<hash>
git-workflow-suggested-file-database-objects-tables-public-country-sql-<hash>
git-workflow-suggested-file-path-database-objects-tables-public-country-sql-<hash>
```

This lets automation correlate:

```text
Git Workflow recommendation
-> Repository Files preview
-> external Git client changed file
```

## Commit Title and Body

DbState currently exposes a distinct commit title and commit body.

Commit title selectors:

- `git-workflow-suggested-commit-title`
- `git-workflow-copy-commit-title`

Commit body selectors:

- `git-workflow-suggested-commit-body`
- `git-workflow-copy-commit-message`

The title element contains only the exact title. The body element preserves line breaks, punctuation, capitalization, repository paths, and object names.

The existing display sections and copy controls remain present. Automation should read the machine-readable elements directly instead of parsing visible prose or command blocks.

## Existing Git Context Selectors

The Git Workflow remains compatible with these context selectors:

- `git-current-branch`
- `git-default-branch`
- `git-protected-branch-status`
- `git-working-tree-status`
- `git-detached-head-status`

## State Behavior

Protected branch:

- `git-workflow-blocked-state` is visible.
- `git-workflow-suggested-branch-name` exposes the exact recommended working branch name.
- Branch guidance remains visible to the user.
- Suggested staging files may be empty until files are generated.

Working branch with no generated changes:

- current branch and worktree status remain visible.
- `git-workflow-no-changes-state` is visible when no DbState files are recommended for staging.

Working branch with generated changes:

- `git-workflow-ready-state` is visible.
- suggested files, commit title, and commit body are populated from DbState's existing recommendation data.

Dirty or conflicting state:

- existing backend guardrail behavior is preserved.
- blocked or error states are exposed through `git-workflow-blocked-state` or `git-workflow-error-state`.

Detached HEAD:

- `git-detached-head-status` shows `yes` when no branch is available from Git status.

Error:

- `git-workflow-error-state` is visible when recommendation refresh fails or the latest operation reports a non-blocking failure.

## Slugging

Dynamic suggested-file selectors reuse the selector slugging documented in `docs/postgresql-v0.1/63-schema-db-to-repo-automation-selectors.md`.

The slug is:

```text
{readable-normalized-value}-{short-stable-hash}
```

The hash suffix is deterministic FNV-1a over JavaScript UTF-16 code units. It is not a runtime hash.

## Safety Boundary

This selector surface does not add:

- Ungit integration
- Playwright dependencies
- branch creation
- branch switching
- file staging
- commits
- pushes
- pulls
- fetches
- tags
- SQL execution
- PostgreSQL mutation
- recommendation algorithm changes
- credential persistence

Passwords, connection URLs, tokens, and environment secrets must not appear in Git Workflow recommendation selectors or recommendation content.
