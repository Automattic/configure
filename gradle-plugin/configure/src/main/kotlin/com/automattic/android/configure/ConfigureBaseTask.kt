package com.automattic.android.configure

import org.gradle.api.DefaultTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskExecutionException
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader

abstract class ConfigureBaseTask: DefaultTask() {
    @Input
    var useLocalBinary = false

    @Input
    var cargoRoot = ""

    @Input
    var configureFilePath = ".configure"

    abstract val command: String

    @Throws(TaskExecutionException::class)
    @org.gradle.api.tasks.TaskAction
    fun runCommand() {

        val processBuilder = ProcessBuilder()

        if(useLocalBinary) {
            processBuilder.directory(File(cargoRoot))
            processBuilder.command(
                    "cargo", "run", command,
                    "--configuration-file-path", configureFilePath,
                    "--force",
                    "-vvvv" // If we're using the `cargo` build, be as verbose as possible
            )
        } else {
            val binaryPath = ConfigureHelpers.configureBinaryPath.toAbsolutePath().toString()
            processBuilder.command(binaryPath, command, "--force")
        }

        val process = processBuilder.start()

        BufferedReader(InputStreamReader(process.inputStream)).use { reader ->
            var line: String?
            while (reader.readLine().also { line = it } != null) {
                println(line)
            }
        }
    }
}