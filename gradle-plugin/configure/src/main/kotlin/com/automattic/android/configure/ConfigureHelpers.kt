package com.automattic.android.configure

import org.gradle.internal.impldep.com.amazonaws.event.SDKProgressPublisher.publishProgress
import java.io.BufferedInputStream
import java.io.File
import java.io.FileOutputStream
import java.net.HttpURLConnection
import java.net.URL
import java.net.URLConnection
import java.nio.file.Path
import java.nio.file.Paths


object ConfigureHelpers {

    val configureRootPath: Path = Paths.get(System.getProperty("user.dir")).resolve("vendor").resolve("configure")
    val configureBinaryPath: Path = this.configureRootPath.resolve("configure")
    val configureZipPath: Path = this.configureRootPath.resolve("configure.zip")

    val configureBinary: File = configureBinaryPath.toFile()

    val pluginUrl: URL
        get() {
            var os = ConfigureHelpers.osType.platform
            println("Detected current OS: $os")
            return URL("https://github.com/Automattic/configure/releases/download/$pluginVersion/configure-$os.zip")
        }

    private enum class OS(val platform: String) {
        WINDOWS("windows"),
        LINUX("linux"),
        MAC("macos"),
        UNKNOWN("not supported"),
    }

    private val osType: OS
        get() {
            val osString = System.getProperty("os.name").toLowerCase()
            if (osString.contains("win")) {
                return OS.WINDOWS
            } else if (osString.contains("nix") || osString.contains("nux")
                    || osString.contains("aix")) {
                return OS.LINUX
            } else if (osString.contains("mac")) {
                return OS.MAC
            }

            return OS.UNKNOWN
        }

    val pluginVersion: String = "0.1.0" // TODO

    fun downloadFile(url: URL, destination: Path) {
        val connection = url.openConnection() as HttpURLConnection
        connection.requestMethod = "GET"
        connection.setRequestProperty("User-Agent", "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.1 Safari/605.1.15")
        connection.connect()

        println(connection.responseCode)
        println(connection.responseMessage)

        val input = BufferedInputStream(url.openStream(), 8192)
        val output = FileOutputStream(destination.toAbsolutePath().toString())

        output.write(input.readBytes())
        output.flush()

        // closing streams
        output.close()
        input.close()
    }
}