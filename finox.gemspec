# frozen_string_literal: true

require_relative "lib/finox/version"

Gem::Specification.new do |spec|
  spec.name = "finox"
  spec.version = Finox::VERSION
  spec.authors = ["kyuuri1791"]

  spec.summary = "MySQL query parser for Ruby, powered by sqlparser-rs."
  spec.description = "Parses MySQL queries with the Rust sqlparser crate (via magnus) " \
                     "and returns the AST as plain Ruby Hashes and Arrays."
  spec.homepage = "https://github.com/kyuuri1791/finox"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.1.0"
  spec.required_rubygems_version = ">= 3.3.11"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = spec.homepage
  spec.metadata["changelog_uri"] = "#{spec.homepage}/blob/main/CHANGELOG.md"
  spec.metadata["rubygems_mfa_required"] = "true"

  # Specify which files should be added to the gem when it is released.
  # The `git ls-files -z` loads the files in the RubyGem that have been added into git.
  gemspec = File.basename(__FILE__)
  spec.files = IO.popen(%w[git ls-files -z], chdir: __dir__, err: IO::NULL) do |ls|
    ls.readlines("\x0", chomp: true).reject do |f|
      (f == gemspec) ||
        f.start_with?(*%w[bin/ test/ spec/ features/ .git .github appveyor Gemfile .rspec .rubocop about.])
    end
  end
  spec.bindir = "exe"
  spec.executables = spec.files.grep(%r{\Aexe/}) { |f| File.basename(f) }
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/finox/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9.128"
end
