# PostgreSQL Edition Git Workflows

Git is the source of truth for desired PostgreSQL database state.

DbState PostgreSQL v0.1 must treat Git workflows as part of database change management.

## Supported Workflows

### Clone Repository

User clones a remote repository through DbState. DbState Service performs the Git operation. The browser UI presents progress and errors.

### Open Repository

User opens an existing local repository. DbState validates the project structure and shows Git status.

### Initialize DbState Structure

User initializes the preferred structure:

```text
database/objects/
database/reference-data/
database/releases/
```

### Show Git Status

DbState shows branch name, changed files, staged files, untracked files, and generated artifacts.

### Branch Creation and Checkout

DbState supports branch creation and checkout with dirty-working-tree warnings.

### Fetch and Pull

DbState supports fetch and pull through the service. Pull should warn when local changes may be affected.

### Stage and Unstage

DbState lets users stage and unstage object files, reference-data files, and generated deployment artifacts.

### Commit

DbState lets users commit selected files after review. It must not auto-commit.

### Push

DbState lets users push after explicit approval. It must not auto-push.

## Dirty Working Tree Warnings

Dirty working tree warnings should appear before compare, sync, plan, pull, checkout, and branch-switching workflows.

## Merge Conflict Handling

Merge conflicts should be detected and surfaced clearly. DbState should identify conflicts in database object files and generated artifacts.

## Safety Rules

DbState must not commit:

- Secrets.
- Credentials.
- Connection passwords.
- Tokens.
- Local-only configuration.
- Local machine paths.
- Unmasked PII.

Git credentials should use safe platform mechanisms such as Git credential helpers, SSH agents, and platform credential stores.
