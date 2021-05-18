# frozen_string_literal: true

require 'ffi'

BINARY_MISSING_MESSAGE = 'The binary could not be found – run `cargo build` in the `configure` project root to build it'

module Configure
  extend FFI::Library

  is_development_environment = File.basename(File.dirname((File.expand_path(__dir__)))) == 'configure'

  lib_name = File.join(__dir__, 'bin', 'libconfigure.dylib')

  if is_development_environment
    puts 'In development mode'
    lib_name = File.join(File.dirname(File.expand_path(__dir__)), 'target', 'debug', 'libconfigure.dylib')
    puts "Using binary at #{lib_name}"
    abort(BINARY_MISSING_MESSAGE) unless File.exist?(lib_name)
    puts 'Binary is present'
  end

  ffi_lib File.expand_path(lib_name, __dir__)
  attach_function :init, [], :void
  attach_function :apply, %i[bool string], :void
  attach_function :update, %i[bool string], :void
end
