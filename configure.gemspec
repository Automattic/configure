require_relative 'ruby-gem/version'

Gem::Specification.new do |s|
  s.name        = 'libconfigure'
  s.version     = Configure::VERSION
  s.date        = '2021-01-22'
  s.summary     = "tbd"
  s.description = "tbd"
  s.authors     = ["automattic"]
  s.email       = 'mobile@automattic.com'
  s.files       = ["ruby-gem/configure.rb"]
  s.homepage    = 'https://rubygems.org/gems/libconfigure'
  s.license     = 'MIT'

  s.require_paths = ["ruby-gem"]

  s.bindir        = 'ruby-gem/bin'
  s.executables   = ['configure_init', 'configure_apply', 'configure_update']

  s.add_dependency 'ffi'
end
