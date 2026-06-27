# Git Integration Principles

Git is the source of truth for desired database state.

DbState should operate directly against Git repositories. Git integration is part of the database change workflow, not a side activity.

## Responsibility split

- DbState Service performs Git operations.
- Browser UI presents Git status, branch selection, commit review, pull and push results, file and object diffs, and conflict warnings.
- CLI exposes equivalent Git-aware workflows for automation.

The browser UI must not perform Git operations directly.

## Required workflows

DbState should support:

- Clone repository.
- Open existing repository.
- Initialize DbState project structure in a repository.
- Show Git status.
- Show changed database object files.
- Show branch name.
- Create branch.
- Checkout branch.
- Fetch.
- Pull.
- Stage files.
- Unstage files.
- Commit changes.
- Push changes.
- Show file and object diff.
- Detect merge conflicts.
- Warn when the working tree is dirty before risky operations.

Dirty working tree warnings should appear before compare, sync, plan, pull, checkout, and branch-switching workflows.

## Safety

DbState should help users commit schema object files and generated deployment artifacts deliberately.

DbState must not auto-push or auto-commit without explicit user approval.

DbState must not commit secrets, credentials, connection passwords, tokens, local-only configuration, local machine paths, or unmasked PII.

Git credentials should use safe platform mechanisms such as existing Git credential helpers, SSH agents, and platform credential stores.

Do not expose Git credentials to browser UI, AI agents, logs, generated reports, or project files.

## Open implementation decision

The exact Git implementation approach is open:

- System Git executable.
- libgit2/git2-rs.
- Hybrid approach.
