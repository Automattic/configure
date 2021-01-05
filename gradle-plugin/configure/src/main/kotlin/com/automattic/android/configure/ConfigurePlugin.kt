package com.automattic.android.configure

import org.gradle.api.Plugin
import org.gradle.api.Project
import java.io.*
import java.nio.file.Path
import java.util.zip.ZipFile

class ConfigurePlugin : Plugin<Project> {

    // Plugin Registration Method (unrelated to `configure apply`)
    override fun apply(target: Project) {

        val applyTask = target.tasks.register("applyConfiguration", ConfigureApplyTask::class.java) {
            this.group = "configure"
            this.description = "Apply the encrypted configuration"
        }

        val updateTask = target.tasks.register("updateConfiguration", ConfigureUpdateTask::class.java) {
            this.group = "configure"
            this.description = "Update the encrypted configuration files from the secrets repository"
        }

        val extension = target.extensions.create("configure", ConfigureExtension::class.java)

        // Copy the extension configuration data into the task
        target.afterEvaluate {
            applyTask.configure {
                this.useLocalBinary = extension.useLocalBinary
                this.cargoRoot = extension.cargoRoot
                this.configureFilePath = extension.configurationFilePath
                this.verboseOutput = extension.verboseOutput
            }

            updateTask.configure {
                this.useLocalBinary = extension.useLocalBinary
                this.cargoRoot = extension.cargoRoot
                this.configureFilePath = extension.configurationFilePath
                this.verboseOutput = extension.verboseOutput
            }

            if(!extension.useLocalBinary) {
                ensureBinaryExists()
                ensureBinaryIsExecutable()
            }
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