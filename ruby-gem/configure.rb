require 'ffi'

module Configure
	extend FFI::Library
	lib_name = "libconfigure.#{::FFI::Platform::LIBSUFFIX}"
	ffi_lib File.expand_path(lib_name, __dir__)
	attach_function :init, [], :void
	attach_function :apply, [:bool, :string], :void
	attach_function :update, [:bool, :string], :void
end
