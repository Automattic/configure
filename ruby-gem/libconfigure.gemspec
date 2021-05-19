# frozen_string_literal: true

require_relative 'version'

Gem::Specification.new do |s|
  s.name        = 'libconfigure'
  s.version     = Configure::VERSION
  s.date        = '2021-01-22'
  s.summary     = 'A lightweight native-backed tool for working with configuration files'
  s.authors     = ['automattic']
  s.email       = 'mobile@automattic.com'
  s.homepage    = 'https://rubygems.org/gems/libconfigure'
  s.license     = 'MIT'
  s.required_ruby_version = '>= 2.6.4'

  s.require_paths = ['.']

  s.bindir        = 'bin'
  s.executables   = [
    'configure'
  ]

  s.files = [
    'configure.rb',
    'version.rb',
    'bin/libconfigure.dylib' # the macOS binary library
  ]

  s.add_dependency('ffi', '~> 1.0')
end
