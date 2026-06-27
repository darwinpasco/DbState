# PostgreSQL v0.1 AI Agent Workflows

DbState PostgreSQL v0.1 should work well with Codex and Claude through deterministic artifacts.

AI is not the correctness layer.

## Allowed Workflows

AI may:

- Review generated SQL synchronization scripts.
- Review risk reports.
- Review dependency warnings.
- Explain selected synchronization plans.
- Generate draft branch names.
- Generate draft commit messages.
- Generate draft PR titles.
- Generate draft PR bodies.
- Generate draft comments.
- Summarize generated artifacts.

## Grounding

AI-generated text must be grounded in DbState artifacts:

- Schema diff.
- Data diff.
- Selected synchronization plan.
- Generated synchronization script.
- Risk report.
- Dependency analysis report.
- Deployment artifact metadata.
- Git status.
- Git branch.
- Git commit hash where available.
- User-selected target environment label.

AI must not invent changes that are not present in the artifacts.

## Approval Boundary

AI-generated text must be editable and user-approved before use.

AI must not:

- Execute SQL.
- Approve database changes.
- Override risk warnings.
- Override dependency warnings.
- Push.
- Post comments.
- Commit.
- Create branches.
- Create PRs.

Any future action-capable integration must require explicit user approval.

## Future MCP Integration

Future MCP integration should expose deterministic, local, bounded operations. MCP must preserve the no-direct-apply rule and credential boundary.
