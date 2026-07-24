# Database Project Initialization Automation Selectors

This note documents the browser automation contract for Cycle 1 of the reference demo, **DbState Demo: Capture and Review PostgreSQL Schema Changes in Git**.

The selectors are for automation addressability only. They do not add Playwright, Ungit integration, Git mutation, PostgreSQL inspection, SQL execution, schema capture, reference-data export, or release artifact generation.

## Starting State Contract

The demo starts with an existing Git repository that is not yet an initialized DbState Database Project:

- current branch: `dev`
- default branch: `dev`
- protected branch: yes
- working tree: clean
- DbState project folders: absent
- saved profile: `Pagila-Local`
- PostgreSQL password: supplied only at runtime

The pre-initialization project status is:

```text
gitRepositoryWithoutDbStateStructure
```

## Initialization Flow

Cycle 1 proves this sequence:

1. The user opens an existing Git repository on protected `dev`.
2. The user enters the required initialization confirmation.
3. DbState rejects initialization because `dev` is protected.
4. No DbState project file or folder is created.
5. Git Workflow remains available and exposes the suggested working branch.
6. Ungit or another external Git client creates and checks out that exact branch.
7. The user refreshes DbState workspace status.
8. DbState shows the recommended non-protected branch as active.
9. The user retries initialization.
10. Initialization succeeds and creates only the supported DbState project structure.
11. Repository Files can show the initialized directories before object SQL files are captured.

Database Project initialization is separate from schema capture. It creates project folders and the default reference-data registry only.

## Initialization Selectors

Controls:

- `workspace-initialize-project`
- `workspace-initialization-panel`
- `workspace-initialization-confirmation-input`
- `workspace-initialization-start`

State:

- `workspace-initialization-status`
- `workspace-initialization-available`
- `workspace-initialization-blocked`
- `workspace-initialization-loading`
- `workspace-initialization-success`
- `workspace-initialization-error`

Error classification:

- `workspace-initialization-error-code`
- `workspace-initialization-error-message`

`workspace-initialization-error-code` contains only the machine-readable classification value. For the protected branch path, the value is:

```text
protectedBranch
```

The readable explanation remains in `workspace-initialization-error-message`.

Project status:

- `workspace-project-status`

The status distinguishes an existing Git repository without DbState structure from an initialized Database Project.

## Git Workflow Integration

Git Workflow remains accessible before initialization because the selected workspace is already a Git repository.

Relevant selectors:

- `git-workflow-panel`
- `git-workflow-status`
- `git-workflow-blocked-state`
- `git-workflow-suggested-branch`
- `git-workflow-suggested-branch-name`
- `git-workflow-branch-reason`
- `git-current-branch`
- `git-default-branch`
- `git-protected-branch-status`
- `git-working-tree-status`
- `git-detached-head-status`

The suggested branch value is read from `git-workflow-suggested-branch-name`. It contains only the branch name, not a shell command.

## Repository Directory Selectors

Repository Files exposes initialized directories with deterministic path-based selectors:

- `repository-directory-row-{repository-path-slug}`
- `repository-directory-path-{repository-path-slug}`
- `repository-directory-name-{repository-path-slug}`

The path element contains only the repository-relative path. The name element contains only the directory name.

Directory selectors use the same `repositoryObjectPathSlug(path)` algorithm used by Repository Files and Git Workflow suggested files:

- normalize separators with `friendlyPath`
- lowercase the readable prefix
- replace non-ASCII-alphanumeric runs with `-`
- trim leading/trailing `-`
- keep a readable prefix up to 72 characters
- append an eight-character deterministic FNV-1a-style hash suffix

The hash suffix prevents collisions when different paths normalize to the same readable prefix.

## Repository Files State Contract

Repository Files exposes:

- `repository-files-status`
- `repository-files-loading`
- `repository-files-loaded`
- `repository-files-error`
- `repository-files-empty-state`
- `repository-files-tree`

The UI distinguishes:

- no DbState project structure: controlled listing error
- initialized Database Project with directories but no captured SQL files: loaded state with directory rows
- initialized Database Project with captured SQL files: loaded state with directory rows and file rows

An initialized project is not treated as empty merely because it has no `.sql` files yet.

## Initialized Directories

The current initializer creates these directories:

- `database`
- `database/objects`
- `database/objects/schemas`
- `database/objects/extensions`
- `database/objects/enums`
- `database/objects/domains`
- `database/objects/aggregates`
- `database/objects/sequences`
- `database/objects/tables`
- `database/objects/indexes`
- `database/objects/views`
- `database/objects/constraints`
- `database/objects/constraints/primary-keys`
- `database/objects/constraints/unique-constraints`
- `database/objects/constraints/foreign-keys`
- `database/objects/constraints/check-constraints`
- `database/objects/materialized-views`
- `database/objects/functions`
- `database/objects/triggers`
- `database/objects/grants`
- `database/objects/grants/schemas`
- `database/objects/grants/tables`
- `database/objects/grants/views`
- `database/objects/grants/materialized-views`
- `database/objects/grants/sequences`
- `database/objects/grants/functions`
- `database/objects/rls-policies`
- `database/reference-data`
- `database/reference-data/tables`
- `database/releases`
- `database/releases/objects`
- `database/releases/reference-data`

The initializer also creates `database/reference-data/dbstate.reference-data.yml`.

## Safety Boundaries

Initialization does not:

- connect to PostgreSQL
- inspect PostgreSQL
- mutate PostgreSQL
- capture schema objects
- export reference data rows
- generate release artifacts
- execute SQL
- stage, commit, push, pull, fetch, branch, checkout, switch, or tag through Git
- persist credentials

DbState blocks initialization on protected branches before creating project files.
