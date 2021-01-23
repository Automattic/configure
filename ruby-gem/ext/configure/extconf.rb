require 'mkmf'
require 'os'
require 'tempfile'
require_relative '../../version'

compilationDirectory = Dir.pwd
configureDirectory = File.dirname(File.absolute_path(__FILE__))
bundlePath = File.join(compilationDirectory, 'configure.' + RbConfig::CONFIG['DLEXT'])

puts "Downloading libconfigure"

FileUtils.cp(configureDirectory + "/makefile.example", compilationDirectory + "/Makefile")

if OS.mac? then
	command = "curl https://github.com/Automattic/configure/releases/download/#{Configure::VERSION}/libconfigure.dylib -L --output #{bundlePath}"
	system(command)
else
	puts "Unsupported Operating System"
	exit 1
end

puts "Done"
exit 0
