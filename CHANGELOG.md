## [0.2.0] - 2026-07-18

- **Breaking:** `Finox.parse` now returns a `Finox::Result` instead of a plain Array.
- Add `Finox::Result#tables`, which returns the tables referenced across all
  parsed statements (including joins, subqueries and DML targets), deduplicated
  and excluding CTE names.
- Add `Finox::Result#columns`, which returns the columns referenced across all
  parsed statements (including `INSERT` column lists and `UPDATE` assignment
  targets), deduplicated.
- Add `Finox::Result#statement_types`, which returns the type of each parsed
  statement (sqlparser's variant names, e.g. `"Query"`, `"Insert"`).
- Add `Finox::Result#select_tables`, `#dml_tables` and `#ddl_tables`, which
  classify the referenced tables into read, written by DML and targeted by DDL
  (a table appearing in multiple roles is listed in each).
- Add `Finox::Result#normalize`, which returns the SQL with literals replaced
  by `?` placeholders, deparsed from the AST (normalizing formatting and
  keyword case).
- Add `Finox::Result#fingerprint`, which returns a 64-bit hex hash of the
  normalized SQL, for grouping queries that differ only in literals or
  formatting. Fingerprints are not guaranteed to be stable across finox
  versions.
- Add `Finox::Result#statements`, which returns the AST of each parsed
  statement as plain Hashes/Arrays. All keys are Strings (previously enum
  variant names were Strings but struct fields were Symbols).
- Precompiled gems now include a shared object for Ruby 4.0.

## [0.1.0] - 2026-07-17

- Initial release
