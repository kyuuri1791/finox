## [Unreleased]

- **Breaking:** `Finox.parse` now returns a `Finox::Result` instead of a plain Array.
- Add `Finox::Result#tables`, which returns the tables referenced across all
  parsed statements (including joins, subqueries and DML targets), deduplicated
  and excluding CTE names.
- Add `Finox::Result#columns`, which returns the columns referenced across all
  parsed statements (including `INSERT` column lists and `UPDATE` assignment
  targets), deduplicated.
- Add `Finox::Result#statement_types`, which returns the type of each parsed
  statement (sqlparser's variant names, e.g. `"Query"`, `"Insert"`).
- Add `Finox::Result#normalize`, which returns the SQL with literals replaced
  by `?` placeholders, deparsed from the AST (normalizing formatting and
  keyword case).
- Add `Finox::Result#statements`, which returns one `Finox::Statement` per
  parsed statement, exposing per-statement `#tables`, `#columns`,
  `#statement_type` and `#normalize` as well as `#to_h` (the AST as plain
  Hashes/Arrays).

## [0.1.0] - 2026-07-17

- Initial release
