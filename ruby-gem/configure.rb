require 'ffi'

module Configure
	extend FFI::Library

	is_development_environment = File.basename(File.dirname((File.expand_path(__dir__)))) == "configure"

	lib_name = File.join(__dir__, "bin", "libconfigure.dylib")

	if is_development_environment
		lib_name = File.join(File.dirname(File.expand_path(__dir__)), "target", "debug", "libconfigure.dylib")
	end

	ffi_lib File.expand_path(lib_name, __dir__)
	attach_function :init, [], :void
	attach_function :apply, [:bool, :string], :void
	attach_function :update, [:bool, :string], :void
end
