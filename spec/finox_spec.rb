# frozen_string_literal: true

RSpec.describe Finox do
  it "has a version number" do
    expect(Finox::VERSION).not_to be nil
  end

  describe ".parse" do
    it "returns an array of statements as hashes" do
      ast = Finox.parse("SELECT id, name FROM users WHERE id = 1")

      expect(ast).to be_an(Array)
      expect(ast.length).to eq(1)
      expect(ast.first).to have_key("Query")
    end

    it "parses MySQL backtick identifiers" do
      ast = Finox.parse("SELECT `id` FROM `users`")

      expect(ast.first).to have_key("Query")
    end

    it "parses multiple statements" do
      ast = Finox.parse("SELECT 1; SELECT 2")

      expect(ast.length).to eq(2)
    end

    it "raises Finox::ParseError for invalid SQL" do
      expect { Finox.parse("SELEKT 1") }.to raise_error(Finox::ParseError, /Expected/)
    end
  end
end
