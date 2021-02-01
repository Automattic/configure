require 'mkmf'
require 'os'
require 'tempfile'
require_relative '../../version'

compilationDirectory = Dir.pwd
configureDirectory = File.dirname(File.absolute_path(__FILE__))
bundlePath = File.join(compilationDirectory, 'configure.' + RbConfig::CONFIG['DLEXT'])

url = "https://github.com/Automattic/configure/releases/download/0.3.0/libconfigure.dylib"

puts "Downloading libconfigure from #{url}"

FileUtils.cp(configureDirectory + "/makefile.example", compilationDirectory + "/Makefile")

if OS.mac? then
	system("curl", "-L", url, "--output", bundlePath)
else
	puts "Unsupported Operating System"
	exit 1
end

puts "Done"
exit 0
