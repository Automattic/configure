require_relative 'ruby-gem/version'

Gem::Specification.new do |s|
  s.name        = 'libconfigure'
  s.version     = Configure::VERSION
  s.date        = '2021-01-22'
  s.summary     = "A lightweight native-backed tool for working with configuration files"
  s.authors     = ["automattic"]
  s.email       = 'mobile@automattic.com'
  s.files       = ["ruby-gem/configure.rb"]
  s.homepage    = 'https://rubygems.org/gems/libconfigure'
  s.license     = 'MIT'

  s.require_paths = ["ruby-gem"]

  s.bindir        = 'ruby-gem/bin'
  s.executables   = ['configure_init', 'configure_apply', 'configure_update']
  s.extensions    = ['ruby-gem/ext/configure/extconf.rb']

  s.add_dependency('ffi', '~> 1.0')
end
