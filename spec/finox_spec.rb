# frozen_string_literal: true

RSpec.describe Finox do
  it "has a version number" do
    expect(Finox::VERSION).not_to be nil
  end

  describe ".parse" do
    it "returns a Finox::Result" do
      expect(Finox.parse("SELECT 1")).to be_a(Finox::Result)
    end

    it "raises Finox::ParseError for invalid SQL" do
      expect { Finox.parse("SELEKT 1") }.to raise_error(Finox::ParseError, /Expected/)
    end
  end

  describe "#tables" do
    it "returns the tables referenced by the query" do
      expect(Finox.parse("SELECT * FROM users").tables).to eq(["users"])
    end

    it "collects tables from joins and subqueries" do
      sql = "SELECT * FROM orders o JOIN users u ON u.id = o.user_id " \
            "WHERE EXISTS (SELECT 1 FROM payments WHERE payments.order_id = o.id)"

      expect(Finox.parse(sql).tables).to eq(%w[orders users payments])
    end

    it "collects tables from INSERT, UPDATE and DELETE" do
      expect(Finox.parse("INSERT INTO logs (msg) VALUES ('x')").tables).to eq(["logs"])
      expect(Finox.parse("UPDATE users SET name = 'a' WHERE id = 1").tables).to eq(["users"])
      expect(Finox.parse("DELETE FROM sessions WHERE id = 1").tables).to eq(["sessions"])
    end

    it "returns backtick identifiers unquoted" do
      expect(Finox.parse("SELECT * FROM `users`").tables).to eq(["users"])
    end

    it "returns qualified names joined with dots" do
      expect(Finox.parse("SELECT * FROM app.users").tables).to eq(["app.users"])
    end

    it "deduplicates repeated tables" do
      expect(Finox.parse("SELECT * FROM users UNION SELECT * FROM users").tables).to eq(["users"])
    end

    it "excludes CTE names" do
      sql = "WITH recent AS (SELECT * FROM orders) SELECT * FROM recent JOIN users ON 1 = 1"

      expect(Finox.parse(sql).tables).to eq(%w[orders users])
    end
  end

  describe "#columns" do
    it "returns the columns referenced by the query" do
      expect(Finox.parse("SELECT id, name FROM users WHERE id = 1").columns).to eq(%w[id name])
    end

    it "collects columns from WHERE, GROUP BY and ORDER BY" do
      sql = "SELECT COUNT(*) FROM users WHERE age > 20 GROUP BY city ORDER BY city"

      expect(Finox.parse(sql).columns).to eq(%w[age city])
    end

    it "returns qualified columns joined with dots" do
      sql = "SELECT u.id FROM users u JOIN orders o ON o.user_id = u.id"

      expect(Finox.parse(sql).columns).to eq(%w[u.id o.user_id])
    end

    it "collects the column list of INSERT" do
      expect(Finox.parse("INSERT INTO logs (msg, level) VALUES ('x', 1)").columns).to eq(%w[msg level])
    end

    it "collects assignment targets of UPDATE" do
      sql = "UPDATE users SET name = 'a', age = age + 1 WHERE id = 1"

      expect(Finox.parse(sql).columns).to eq(%w[name age id])
    end

    it "returns backtick identifiers unquoted" do
      expect(Finox.parse("SELECT `name` FROM users").columns).to eq(["name"])
    end

    it "does not include wildcards" do
      expect(Finox.parse("SELECT * FROM users").columns).to eq([])
    end
  end

  describe "#statement_types" do
    it "returns the type of each statement" do
      sql = "SELECT 1; INSERT INTO logs (msg) VALUES ('x'); " \
            "UPDATE users SET name = 'a'; DELETE FROM users"

      expect(Finox.parse(sql).statement_types).to eq(%w[Query Insert Update Delete])
    end
  end

  describe "#normalize" do
    it "replaces literals with placeholders" do
      sql = "SELECT * FROM users WHERE id = 1 AND name = 'foo'"

      expect(Finox.parse(sql).normalize).to eq("SELECT * FROM users WHERE id = ? AND name = ?")
    end

    it "normalizes formatting and keyword case" do
      expect(Finox.parse("select  *\nfrom users where id=1").normalize)
        .to eq("SELECT * FROM users WHERE id = ?")
    end

    it "replaces each element of IN lists" do
      expect(Finox.parse("SELECT * FROM users WHERE id IN (1, 2, 3)").normalize)
        .to eq("SELECT * FROM users WHERE id IN (?, ?, ?)")
    end

    it "joins multiple statements with semicolons" do
      expect(Finox.parse("SELECT 1; SELECT 2").normalize).to eq("SELECT ?; SELECT ?")
    end
  end

  describe "#statements" do
    it "returns one Finox::Statement per statement" do
      statements = Finox.parse("SELECT 1; SELECT 2").statements

      expect(statements).to be_an(Array)
      expect(statements.length).to eq(2)
      expect(statements).to all(be_a(Finox::Statement))
    end
  end
end

RSpec.describe Finox::Statement do
  let(:statements) { Finox.parse("SELECT hoge FROM table1; SELECT fuga FROM table2").statements }

  describe "#tables" do
    it "returns the tables per statement" do
      expect(statements.map(&:tables)).to eq([["table1"], ["table2"]])
    end
  end

  describe "#columns" do
    it "returns the columns per statement" do
      expect(statements.map(&:columns)).to eq([["hoge"], ["fuga"]])
    end
  end

  describe "#statement_type" do
    it "returns the statement's type" do
      expect(Finox.parse("SELECT 1").statements.first.statement_type).to eq("Query")
    end
  end

  describe "#normalize" do
    it "returns the normalized SQL of the single statement" do
      statements = Finox.parse("SELECT 1; SELECT * FROM users WHERE id = 2").statements

      expect(statements.map(&:normalize)).to eq(["SELECT ?", "SELECT * FROM users WHERE id = ?"])
    end
  end

  describe "#to_h" do
    it "returns the statement AST as a Hash" do
      statement = Finox.parse("SELECT `id` FROM `users`").statements.first

      expect(statement.to_h).to be_a(Hash)
      expect(statement.to_h).to have_key("Query")
    end
  end
end
