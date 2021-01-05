package com.automattic.android.configure

import org.gradle.api.DefaultTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskExecutionException
import java.io.File

abstract class ConfigureBaseTask: DefaultTask() {
    @Input
    var useLocalBinary = false

    @Input
    var cargoRoot = ""

    @Input
    var configureFilePath = ".configure"

    @Input
    var verboseOutput = false

    @get:Input
    abstract val command: String

    @Throws(TaskExecutionException::class)
    @org.gradle.api.tasks.TaskAction
    fun runCommand() {

        if(useLocalBinary) {
            project.exec {
                workingDir = File(cargoRoot)

                var args = mutableListOf(
                    "cargo", "run", command,
                    "--configuration-file-path", configureFilePath,
                    "--force"
                )

                if(verboseOutput) {
                    args.add("-vvvv")
                }

                commandLine = args
            }
        } else {
            val binaryPath = ConfigureHelpers.configureBinaryPath.toAbsolutePath().toString()

            project.exec {
                var args = mutableListOf(binaryPath, command, "--force")

                if(verboseOutput) {
                    args.add("-vvvv")
                }

                commandLine = args
            }
        }
    }
}