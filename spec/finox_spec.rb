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

    it "does not apply CTE names across statements" do
      sql = "SELECT * FROM users; WITH users AS (SELECT 1) SELECT * FROM users"

      expect(Finox.parse(sql).tables).to eq(["users"])
    end
  end

  describe "#select_tables" do
    it "returns the tables read from" do
      expect(Finox.parse("SELECT * FROM users JOIN orders ON 1 = 1").select_tables).to eq(%w[users orders])
    end

    it "excludes tables only written to" do
      expect(Finox.parse("INSERT INTO logs SELECT * FROM events").select_tables).to eq(["events"])
      expect(Finox.parse("DELETE FROM sessions WHERE id = 1").select_tables).to eq([])
    end

    it "includes source tables of DML" do
      sql = "UPDATE users SET name = 'a' WHERE id IN (SELECT user_id FROM admins)"

      expect(Finox.parse(sql).select_tables).to eq(["admins"])
    end

    it "includes tables both read and written" do
      expect(Finox.parse("INSERT INTO users SELECT * FROM users").select_tables).to eq(["users"])
    end
  end

  describe "#dml_tables" do
    it "returns the tables written to by INSERT, UPDATE and DELETE" do
      expect(Finox.parse("INSERT INTO logs (msg) VALUES ('x')").dml_tables).to eq(["logs"])
      expect(Finox.parse("UPDATE users SET name = 'a'").dml_tables).to eq(["users"])
      expect(Finox.parse("DELETE FROM sessions").dml_tables).to eq(["sessions"])
    end

    it "excludes tables only read from" do
      expect(Finox.parse("INSERT INTO logs SELECT * FROM events").dml_tables).to eq(["logs"])
      expect(Finox.parse("SELECT * FROM users").dml_tables).to eq([])
    end

    it "returns only the deleted tables of a multi-table DELETE" do
      expect(Finox.parse("DELETE t1 FROM t1 JOIN t2 ON t1.id = t2.id").dml_tables).to eq(["t1"])
    end
  end

  describe "#ddl_tables" do
    it "returns the tables targeted by DDL" do
      expect(Finox.parse("CREATE TABLE t (id INT)").ddl_tables).to eq(["t"])
      expect(Finox.parse("ALTER TABLE users ADD COLUMN age INT").ddl_tables).to eq(["users"])
      expect(Finox.parse("DROP TABLE users, sessions").ddl_tables).to eq(%w[users sessions])
      expect(Finox.parse("TRUNCATE TABLE logs").ddl_tables).to eq(["logs"])
    end

    it "excludes tables only read from" do
      expect(Finox.parse("CREATE TABLE t2 AS SELECT * FROM t1").ddl_tables).to eq(["t2"])
      expect(Finox.parse("SELECT * FROM users").ddl_tables).to eq([])
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

  describe "#fingerprint" do
    it "ignores differences in literals and formatting" do
      expect(Finox.parse("SELECT * FROM users WHERE id = 1").fingerprint)
        .to eq(Finox.parse("select *  from users\nwhere id=42").fingerprint)
    end

    it "differs for structurally different queries" do
      expect(Finox.parse("SELECT * FROM users").fingerprint)
        .not_to eq(Finox.parse("SELECT * FROM orders").fingerprint)
    end

    it "returns a 16-character hex string" do
      expect(Finox.parse("SELECT 1").fingerprint).to match(/\A[0-9a-f]{16}\z/)
    end
  end

  describe "#statements" do
    it "returns the AST of each statement as plain Hashes" do
      statements = Finox.parse("SELECT 1; SELECT 2").statements

      expect(statements).to be_an(Array)
      expect(statements.length).to eq(2)
      expect(statements).to all(be_a(Hash).and(have_key("Query")))
    end

    it "uses String keys for variant names and fields alike" do
      ast = Finox.parse("SELECT id FROM users").statements.first

      expect(ast.dig("Query", "body", "Select", "projection").length).to eq(1)
    end
  end
end
