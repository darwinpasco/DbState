# Product Line Strategy

DbState should become a product family with native RDBMS editions.

## Native editions

Each database engine has its own object model and operational risks. A PostgreSQL adapter should not pretend PostgreSQL is the same as SQL Server, MySQL, SQLite, Db2, or Access.

Each edition should respect:

- Native SQL dialect.
- Native object model.
- Native dependency rules.
- Native deployment behavior.
- Native permissions and security model.
- Native operational risks.

## Planned editions

- DbState PostgreSQL, planned first.
- DbState MySQL.
- DbState SQL Server.
- DbState SQLite.
- DbState Db2.
- DbState Access.

This repository remains the upstream/base product repository for now. Future editions may be forked, split, or packaged separately when the time is right. That repository strategy is an open decision.

## Operating systems are not product lines

Windows, macOS, and Linux are distribution targets. They are not separate product lines.

Correct examples:

- DbState PostgreSQL for Windows.
- DbState PostgreSQL for macOS.
- DbState PostgreSQL for Linux.
- DbState PostgreSQL Docker image.

Incorrect examples:

- DbState Windows.
- DbState macOS.
- DbState Linux.

The product line should split by RDBMS, not by operating system.
