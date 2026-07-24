# Schema Database-to-Repository Automation Selectors

This note documents stable browser automation selectors for the `Schema Compare: Database to Repository` capture workflow.

The selectors are for browser automation only. They do not add Playwright, video automation, branch creation, Git mutation, SQL execution, or repository-write behavior.

Git Workflow recommendation selectors are documented separately in `docs/postgresql-v0.1/64-git-workflow-automation-selectors.md`.

Database Project initialization selectors for the first demo cycle are documented in `docs/postgresql-v0.1/65-database-project-initialization-automation-selectors.md`.

## Workflow

The selector contract covers this demo path:

```text
Open workspace
-> Select Schema Compare: Database to Repository
-> Select saved profile
-> Enter session-only password
-> Test connection
-> Preview repository sync
-> Filter and inspect objects
-> Select objects
-> View Object Diff
-> Confirm repository write
-> View generated files in Repository Files
```

The workflow mode value is:

```text
schemaDatabaseToRepository
```

The visible label remains:

```text
Schema Compare: Database to Repository
```

## Selector Inventory

Workspace:

- `workspace-path-input`
- `workspace-browse`
- `workspace-check-status`
- `workspace-status`
- `workspace-git-branch`
- `workspace-protected-branch`
- `workspace-project-status`

Directory picker:

- `directory-picker`
- `directory-picker-path-input`
- `directory-picker-parent`
- `directory-picker-entry-{stable-directory-path-slug}`
- `directory-select-current`
- `directory-picker-cancel`
- `directory-picker-error`

Workflow:

- `workflow-mode`
- `active-workflow-mode`

Connection:

- `connection-mode`
- `profile-select`
- `profile-password-input`
- `source-target-run`
- `database-connection-status`
- `database-connection-success`
- `database-connection-error`

Compare execution:

- `tab-compare-options`
- `preview-repository-sync`
- `repository-sync-preview-status`
- `repository-sync-preview-loading`
- `repository-sync-preview-success`
- `repository-sync-preview-error`
- `comparison-summary`

Results:

- `results-table`
- `results-object-type-filter`
- `results-status-filter`
- `results-search-input`
- `results-count`
- `results-empty-state`
- `results-row-{object-ref-slug}`
- `results-row-status-{object-ref-slug}`
- `results-row-type-{object-ref-slug}`
- `results-row-path-{object-ref-slug}`
- `results-include-{object-ref-slug}`
- `results-open-{object-ref-slug}`

Object Diff:

- `tab-object-diff`
- `object-diff-panel`
- `object-diff-object-ref`
- `object-diff-object-type`
- `object-diff-status`
- `object-diff-database`
- `object-diff-repository`
- `object-diff-loading`
- `object-diff-error`

Repository write:

- `repository-write-confirmation-input`
- `write-repository-changes`
- `repository-write-preflight`
- `repository-write-loading`
- `repository-write-success`
- `repository-write-error`
- `repository-write-written-count`
- `repository-write-skipped-count`

Git status:

- `tab-git-workflow`
- `git-current-branch`
- `git-default-branch`
- `git-protected-branch-status`
- `git-working-tree-status`
- `git-detached-head-status`

Repository Files:

- `repository-files-tab`
- `repository-files-panel`
- `repository-files-refresh`
- `repository-files-tree`
- `repository-files-empty-state`
- `repository-file-selected-path`
- `repository-file-preview`
- `repository-file-preview-loading`
- `repository-file-preview-error`
- `repository-file-preview-read-only`
- `repository-file-row-{repository-path-slug}`

## Slugging Rule

Dynamic selectors use:

```text
{readable-normalized-value}-{short-stable-hash}
```

Readable normalization:

1. Convert to lowercase.
2. Replace non-ASCII-alphanumeric characters with `-`.
3. Trim leading and trailing `-`.
4. Limit the readable prefix to 72 characters.
5. Use a fallback such as `result`, `directory`, or `repository-object` when no readable characters remain.

The hash suffix is deterministic FNV-1a over the JavaScript UTF-16 code units. It is not Rust's runtime hash and does not vary between process executions.

This handles schemas, object types, dots, slashes, backslashes, spaces, quotes, parentheses, mixed case, and PostgreSQL identifiers containing punctuation. Normalized collisions are disambiguated by the stable hash suffix.

Examples:

```text
table:public.country
results-row-table-public-country-<hash>

database/objects/tables/public.actor.sql
repository-file-row-database-objects-tables-public-actor-sql-<hash>

C:\DbState\Your Database Repo
directory-picker-entry-c-dbstate-your-database-repo-<hash>
```

## Waiting Guidance

Automation should wait for explicit success or error selectors instead of fixed sleeps:

- Connection: `database-connection-success` or `database-connection-error`
- Repository sync preview: `repository-sync-preview-success` or `repository-sync-preview-error`
- Repository write: `repository-write-success` or `repository-write-error`
- Object Diff: `object-diff-loading` hidden and `object-diff-error` hidden or visible
- Repository Files preview: `repository-file-preview-loading` hidden and `repository-file-preview-error` hidden or visible

## Credential Safety

The session-only password input has the fixed selector `profile-password-input`.

Selector code must not derive `data-testid` values from password text. The password value must not appear in status elements, generated selectors, UI logs, backend logs, or persisted profile data.

The saved profile selector can choose a profile such as `Pagila-Local`, but the profile password remains a session-only browser input.

## Safety Boundaries

These selectors do not add:

- Git branch creation or switching
- Git add, commit, push, pull, fetch, tag, checkout, or branch execution
- SQL execution
- PostgreSQL mutation
- repository-write bypasses
- password persistence
- Playwright dependencies
- demo scripts
