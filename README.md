# Finox

A MySQL query parser for Ruby, powered by [sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs).

Finox parses SQL with Rust and exposes tables, columns, fingerprints and the raw AST.

## Installation

```bash
bundle add finox
```

Or without Bundler:

```bash
gem install finox
```

Precompiled native gems are available for Ruby 3.1–4.0 on the following
platforms, so installation there requires no Rust toolchain:

| OS      | Platforms                                        |
| ------- | ------------------------------------------------ |
| Linux   | x86_64, aarch64 (glibc and musl variants)        |
| macOS   | x86_64, arm64                                    |
| Windows | x64-mingw-ucrt                                   |

On any other platform the source gem is installed instead, and compiling it
requires a Rust toolchain.

## Usage

`Finox.parse` returns a `Finox::Result`:

```ruby
require "finox"

result = Finox.parse("SELECT `name` FROM `users` WHERE id = 1")
# => #<Finox::Result>

result.tables          # => ["users"]
result.columns         # => ["name", "id"]
result.statement_types # => ["Query"]
result.normalize       # => "SELECT `name` FROM `users` WHERE id = ?"
result.fingerprint     # => "24e307ca0f02abfc"
```

Invalid SQL raises `Finox::ParseError`:

```ruby
Finox.parse("SELEKT 1")
# => Finox::ParseError: sql parser error: Expected: an SQL statement, found: SELEKT at Line: 1, Column: 1
```

### Table classification

`#select_tables`, `#dml_tables` and `#ddl_tables` classify the referenced
tables by how they are used: read, written by DML (`INSERT` / `UPDATE` /
`DELETE`) or targeted by DDL (`CREATE TABLE` / `ALTER TABLE` / `DROP TABLE` /
`TRUNCATE` etc.). A table appearing in multiple roles is listed in each.

```ruby
result = Finox.parse("INSERT INTO logs SELECT message FROM events; DROP TABLE archives")

result.tables        # => ["logs", "events", "archives"]
result.select_tables # => ["events"]
result.dml_tables    # => ["logs"]
result.ddl_tables    # => ["archives"]
```

#### Known limitations

Extraction is syntactic, with no schema knowledge and only coarse scope
resolution:

- A CTE name shadows a same-named real table everywhere in the statement,
  even inside the CTE's own definition.
- Multi-table `UPDATE` (`UPDATE t1 JOIN t2 ... SET t2.x = 1`) reports only
  `t1` in `#dml_tables`.
- `#columns` does not resolve table aliases (`u.id` stays `u.id`).

### Normalization and fingerprints

`#normalize` replaces literals with `?` placeholders and deparses from the
AST, so formatting and keyword case are normalized as well. `#fingerprint` is
a 64-bit hex hash (xxhash64) of the normalized SQL — stable across literal
and formatting differences, but not guaranteed to be stable across finox
versions, since the normalized form depends on the bundled sqlparser-rs.
Recompute stored fingerprints when upgrading finox.

```ruby
Finox.parse("select * from users where id=1").normalize
# => "SELECT * FROM users WHERE id = ?"

Finox.parse("select * from users where id=1").fingerprint ==
  Finox.parse("SELECT * FROM users WHERE id = 42").fingerprint
# => true
```

### Raw sqlparser-rs AST

For anything not covered above, `#statements` exposes sqlparser-rs's raw AST of each parsed statement as plain
Ruby Hashes and Arrays:

```ruby
ast = Finox.parse("SELECT `name` FROM `users` WHERE id = 1").statements.first
# => {"Query" => {"with" => nil, "body" => {"Select" => {...}}, ...}}

ast.dig("Query", "body", "Select", "projection")
# => [{"UnnamedExpr" =>
#       {"Identifier" =>
#         {"value" => "name",
#          "quote_style" => "`",
#          "span" => {"start" => {"line" => 1, "column" => 8},
#                     "end" => {"line" => 1, "column" => 14}}}}}]
```

## Development

```bash
bundle install
bundle exec rake compile  # build the Rust extension
bundle exec rake          # compile + spec + rubocop
```

## License

[MIT](LICENSE.txt)

Precompiled gems statically link Rust crates, including sqlparser-rs
(Apache-2.0). See [LICENSE-THIRD-PARTY.txt](LICENSE-THIRD-PARTY.txt) for
their licenses and attributions.
