# Finox

A MySQL query parser for Ruby, powered by [sqlparser-rs](https://github.com/apache/datafusion-sqlparser-rs).

Finox parses SQL with Rust and returns the AST as plain Ruby Hashes and Arrays.

## Installation

Add to your Gemfile:

```bash
bundle add finox
```

Or install directly:

```bash
gem install finox
```

## Usage

```ruby
require "finox"

ast = Finox.parse("SELECT `name` FROM `users` WHERE id = 1")
# => [{"Query" => {with: nil, body: {"Select" => {...}}, ...}}]

ast.dig(0, "Query", :body, "Select", :projection)
# => [{"UnnamedExpr" =>
#       {"Identifier" =>
#         {value: "name",
#          quote_style: "`",
#          span: {start: {line: 1, column: 8}, end: {line: 1, column: 14}}}}}]
```

`Finox.parse` returns one Hash per statement, so `"SELECT 1; SELECT 2"` yields an array of two.

Key convention (from serde's externally tagged enums): enum variant names are `String` keys (`"Query"`, `"Select"`, `"Identifier"`), struct fields are `Symbol` keys (`:body`, `:projection`, `:value`).

Invalid SQL raises `Finox::ParseError`:

```ruby
Finox.parse("SELEKT 1")
# => Finox::ParseError: sql parser error: Expected: an SQL statement, found: SELEKT at Line: 1, Column: 1
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
