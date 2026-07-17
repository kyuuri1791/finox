# frozen_string_literal: true

require_relative "finox/version"

begin
  # Precompiled native gems ship one shared object per minor Ruby version.
  ruby_minor = RUBY_VERSION[/\d+\.\d+/]
  require_relative "finox/#{ruby_minor}/finox"
rescue LoadError
  require_relative "finox/finox"
end
