# PostgreSQL v0.1 Commercial Packaging Alignment

Commercial tiering is future packaging guidance. This documentation task does not implement gating.

Direct target-database apply is not available in any tier.

## Free

Free should include enough PostgreSQL capability to prove the product:

- Local individual use.
- Visual schema compare.
- Basic Git integration.
- Database-to-repo synchronization.
- Repo-to-database script generation only.
- Version-controlled release scripts.
- Cherry-picking.
- Dependency warnings.
- Basic risk report.
- Limited reference-data compare.
- Basic CLI.
- Basic AI-assisted branch and PR text.

These capabilities align with the v0.1 MVP direction.

## Professional

Professional may add power-user capability:

- Unlimited local projects.
- Full local visual compare.
- Advanced reference-data compare.
- Enhanced risk reports.
- Better dependency explanations.
- Full CLI and Docker.
- Exportable reports.
- Full AI-assisted workflow text.
- Deployment Rehearsal, unless later moved higher.

## Team

Team may add collaboration:

- Multi-user project sharing.
- Shared environment definitions.
- Scheduled drift checks.
- PR integration.
- Draft PR comments.
- Lightweight approvals.
- Team dashboard.
- Notifications.
- Deployment Rehearsal.

## Enterprise

Enterprise may add governance:

- Self-hosted DbState Server.
- RBAC.
- SSO, SAML, and OIDC.
- Audit logs.
- Compliance evidence packs.
- Policy-as-code.
- Advanced approval gates.
- Environment promotion map.
- Continuous drift monitoring.
- Central project registry.
- Enterprise support.
- Air-gapped option later.

## MVP and Later

MVP focuses on local PostgreSQL workflows, Git integration, compare, repository synchronization, script generation, reference-data basics, CLI JSON, Docker design, and AI-assisted draft text.

Deployment Rehearsal is premium later, not MVP.

Direct target-database apply remains unavailable in Free, Professional, Team, and Enterprise.
