# PostgreSQL v0.1 Open Decisions

The following decisions remain unresolved:

- Supported PostgreSQL versions.
- Exact Rust web framework: Axum, Actix Web, or another option.
- Exact UI framework.
- Exact local service port strategy.
- Exact credential store strategy.
- Exact Git implementation approach: system Git, libgit2/git2-rs, or hybrid.
- Exact release artifact naming convention.
- YAML versus JSON reference-data files, or support for both.
- Exact CLI command names and option names.
- Exact CLI JSON schemas.
- Whether MCP is MVP or post-MVP.
- Which PostgreSQL edge cases are deferred.
- Whether Docker image ships in v0.1 or immediately after.
- Packaging strategy for Windows, macOS, and Linux.
- Function signature file naming.
- Extension-owned object handling.
- Partitioned table support.
- Object comment support.
- RLS policy support.
- Grant and role modeling depth.
- Whether service API routes are public, internal, or both in v0.1.
- Whether generated SQL script headers have a strict schema in v0.1.
- Exact risk severity taxonomy.
- Exact dependency override policy.
