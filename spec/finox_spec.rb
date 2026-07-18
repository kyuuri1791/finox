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

  describe "#statements" do
    it "returns an array of statements as hashes" do
      statements = Finox.parse("SELECT id, name FROM users WHERE id = 1").statements

      expect(statements).to be_an(Array)
      expect(statements.length).to eq(1)
      expect(statements.first).to have_key("Query")
    end

    it "parses MySQL backtick identifiers" do
      statements = Finox.parse("SELECT `id` FROM `users`").statements

      expect(statements.first).to have_key("Query")
    end

    it "parses multiple statements" do
      statements = Finox.parse("SELECT 1; SELECT 2").statements

      expect(statements.length).to eq(2)
    end
  end
end
