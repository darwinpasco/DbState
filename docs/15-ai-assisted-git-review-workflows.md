# AI-Assisted Git and Review Workflows

DbState should support AI-assisted generation of Git and review workflow text.

This is an assistant feature, not a source-of-truth feature.

## Allowed assistance

AI may help generate:

- Branch names.
- Commit messages.
- PR titles.
- PR bodies.
- Review comments.
- Object-level change explanations.
- Risk summary comments.
- Dependency warning comments.
- Deployment artifact summaries.
- Deployment rehearsal summaries where applicable.

AI-generated text must be based on deterministic DbState artifacts such as schema diff, data diff, selected synchronization plan, generated synchronization script, risk report, dependency analysis report, deployment artifact metadata, rehearsal report, Git status, Git branch, Git commit hash, and user-selected target environment label.

AI-generated text must be editable before use.

## Required boundaries

AI must not invent changes that are not in the diff, risk report, dependency report, synchronization script, or rehearsal report.

DbState must not automatically create branches, commit, push, create PRs, or post comments without explicit user approval.

AI-generated comments should remain drafts until approved by the user.

AI must not approve database changes, override dependency warnings, hide destructive-change warnings, or execute SQL.

DbState's deterministic engine remains the authority for schema diff, data diff, risk classification, dependency analysis, and synchronization planning.

## PR body content

AI-assisted PR bodies should include:

- Summary.
- Source and target context.
- Changed objects.
- Selected synchronization scope.
- Reference-data changes if any.
- Generated synchronization script path.
- Risk classification.
- Dependency warnings.
- Destructive changes if any.
- Deployment rehearsal result if available.
- Human review checklist.
- Note that DbState does not apply the script directly to the target database.
