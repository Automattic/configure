package com.automattic.android.configure

import org.gradle.api.DefaultTask
import org.gradle.api.tasks.TaskExecutionException
import java.io.BufferedReader
import java.io.InputStreamReader


open class ConfigureApplyTask : DefaultTask() {
    @Throws(TaskExecutionException::class)
    @org.gradle.api.tasks.TaskAction
    fun configureApply() {

        val processBuilder = ProcessBuilder()
        processBuilder.command(ConfigureHelpers.configureBinaryPath.toAbsolutePath().toString(), "apply", "--force")

        val process = processBuilder.start()

        BufferedReader(InputStreamReader(process.inputStream)).use { reader ->
            var line: String?
            while (reader.readLine().also { line = it } != null) {
                println(line)
            }
        }
    }
}
