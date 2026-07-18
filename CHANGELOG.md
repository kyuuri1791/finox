## [Unreleased]

- **Breaking:** `Finox.parse` now returns a `Finox::Result` instead of a plain Array.
  The AST is available as plain Hashes/Arrays via `Finox::Result#statements`.
- Add `Finox::Result#tables`, which returns the tables referenced by the parsed
  statements (including joins, subqueries and DML targets), deduplicated and
  excluding CTE names.

## [0.1.0] - 2026-07-17

- Initial release
