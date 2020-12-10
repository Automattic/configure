package com.automattic.android.configure

import org.gradle.api.Plugin
import org.gradle.api.Project
import java.io.*
import java.nio.file.Path
import java.util.zip.ZipFile

class ConfigurePlugin : Plugin<Project> {

    override fun apply(target: Project) {
        ensureBinaryExists()
        ensureBinaryIsExecutable()

        target.tasks.register("applyConfiguration", ConfigureApplyTask::class.java) {
            this.group = "configure"
            this.description = "Apply the encrypted configuration"
        }
    }

    private fun ensureBinaryIsExecutable() {
        if (!ConfigureHelpers.configureBinary.canExecute())  {
            ConfigureHelpers.configureBinary.setExecutable(true)
        }
    }

    private fun ensureBinaryExists() {
        println("Checking whether `configure` binary is present")
        if (ConfigureHelpers.configureBinary.exists() && ConfigureHelpers.configureBinary.isFile) {
            return
        }

        // Create the storage directory if it doesn't already exist
        if (!ConfigureHelpers.configureBinary.exists()) {
            ConfigureHelpers.configureRootPath.toFile().mkdirs()
        }

        println("Downloading `configure` binary")

        if (!ConfigureHelpers.configureZipPath.toFile().exists()) {
            ConfigureHelpers.downloadFile(ConfigureHelpers.pluginUrl, ConfigureHelpers.configureZipPath)
        }

        unzip(ConfigureHelpers.configureZipPath, ConfigureHelpers.configureRootPath)
    }

    @Throws(IOException::class)
    private fun unzip(source: Path, destination: Path) {

        ZipFile(source.toFile()).use { zip ->
            zip.entries().asSequence().forEach { entry ->
                zip.getInputStream(entry).use { input ->
                    val fileDestination = destination.resolve(entry.name)
                    fileDestination.toFile().outputStream().use { output ->
                        input.copyTo(output)
                    }
                }
            }
        }
    }
}