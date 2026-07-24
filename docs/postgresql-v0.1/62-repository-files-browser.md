# Repository Files Browser

DbState now includes a read-only browser page named `Repository Files` for inspecting generated PostgreSQL desired-state object files.

## Scope

The page lists and previews files only under:

```text
database/objects/
```

The first version supports `.sql` files only. It is intended for the Database-to-Repository workflow, where DbState writes generated desired-state object files under `database/objects/` after explicit user confirmation.

## Read-Only Behavior

The Repository Files page can:

- refresh the controlled `database/objects/` listing
- show directories and SQL files in deterministic order
- show the selected repository-relative path
- preview SQL file contents as read-only text

The page cannot:

- edit, save, rename, move, or delete files
- execute SQL
- apply generated SQL to PostgreSQL
- mutate PostgreSQL
- stage, commit, push, pull, fetch, tag, switch, checkout, or branch Git
- preview files under `database/releases/`
- browse arbitrary workspace directories

Release artifact preview remains a separate capability scoped to `database/releases/`.

## Path Safety

Repository file listing and preview are implemented as separate local service operations:

```text
POST /api/v1/repository/object-files/list
POST /api/v1/repository/object-files/preview
```

Both operations resolve the selected workspace through the existing DbState workspace handling and require it to be inside a local Git working tree.

Preview requests must supply a repository-relative path under `database/objects/`. The service rejects:

- parent traversal such as `..`
- current-directory traversal such as `.`
- absolute paths
- drive-qualified paths
- UNC paths
- mixed-separator traversal
- percent-encoded traversal markers
- files outside `database/objects/`
- files outside the controlled root after canonical path resolution
- files without a `.sql` extension
- directories
- missing files

Symlinks, junctions, or reparse points are not allowed to escape the canonical `database/objects/` root.

## Preview Size Limit

The SQL preview limit is `1 MiB`.

If a file is larger than that limit, DbState returns a controlled error instead of loading the file into browser memory.

## Stable UI Selectors

The Repository Files UI exposes stable selectors for future browser automation:

```text
repository-files-tab
repository-files-panel
repository-files-refresh
repository-files-tree
repository-files-empty-state
repository-file-selected-path
repository-file-preview
repository-file-preview-loading
repository-file-preview-error
repository-file-preview-read-only
```

File rows use:

```text
repository-file-row-{stable-path-slug}
```

Slugging rule:

1. Start with the repository-relative path, for example `database/objects/tables/public.actor.sql`.
2. Lowercase ASCII letters.
3. Replace every non-ASCII-alphanumeric character with `-`.
4. Collapse repeated separators by removing empty segments.
5. Trim leading and trailing separators.

Example:

```text
database/objects/tables/public.actor.sql
```

becomes:

```text
repository-file-row-database-objects-tables-public-actor-sql
```

Selectors are derived from repository-relative paths, not array indexes, screen position, CSS classes, visible SQL text, or runtime-generated identifiers.
