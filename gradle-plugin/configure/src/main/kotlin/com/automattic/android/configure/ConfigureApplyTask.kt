package com.automattic.android.configure

import org.gradle.api.DefaultTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskExecutionException
import java.io.BufferedReader
import java.io.File
import java.io.InputStreamReader

open class ConfigureApplyTask : DefaultTask() {

    @Input var useLocalBinary = false

    @Input var cargoRoot = ""

    @Throws(TaskExecutionException::class)
    @org.gradle.api.tasks.TaskAction
    fun configureApply() {

        var binaryPath = ConfigureHelpers.configureBinaryPath.toAbsolutePath().toString()
        val processBuilder = ProcessBuilder()

        if(useLocalBinary) {
            processBuilder.directory(File(cargoRoot))
            binaryPath = "cargo"
        }

        processBuilder.command(binaryPath, "run", "apply", "--force")

        val process = processBuilder.start()

        BufferedReader(InputStreamReader(process.inputStream)).use { reader ->
            var line: String?
            while (reader.readLine().also { line = it } != null) {
                println(line)
            }
        }
    }
}
