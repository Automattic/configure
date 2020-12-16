import com.novoda.gradle.release.PublishExtension
import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

/// The plugin version number – change this to match whatever your tag will be
version = "0.2.1"
group = "com.automattic.android"

buildscript {
    repositories {
        jcenter()
        maven {
            setUrl("https://plugins.gradle.org/m2/")
        }
    }
    dependencies {
        classpath("com.novoda", "bintray-release", "0.9.2")
    }
}

plugins {
    `kotlin-dsl`
    id("com.github.gmazzo.buildconfig") version "2.0.2"
}

apply(null, "com.novoda.bintray-release")

repositories {
    jcenter()
}

/// Don't allow warnings in the project – this can prevent us from shipping a broken build
tasks.withType<KotlinCompile> {
    kotlinOptions.allWarningsAsErrors = true
}

/// Target the v1.8 JVM
val compileKotlin: KotlinCompile by tasks
compileKotlin.kotlinOptions {
    jvmTarget = "1.8"
}

/// Disable a warning about the `kotlin-dsl` using experimental Kotlin features
kotlinDslPluginOptions {
    experimentalWarning.set(false)
}

/// Register the plugin with Gradle – this is instead of using a `META-INF/gradle-plugins` directory
gradlePlugin {
    plugins {
        create("configure") {
            id = "com.automattic.android.configure"
            implementationClass = "com.automattic.android.configure.ConfigurePlugin"
        }
    }
}

/// Register the plugin's maven configuration for upload
configure<PublishExtension> {
    userOrg = "automattic"
    groupId = "com.automattic.android"
    artifactId = "configure"
    publishVersion = "${version}"
    desc = "A lightweight tool for working with configuration files"
    website = "https://github.com/automattic/configure"

    dryRun = false
    autoPublish = true

    bintrayUser = System.getenv("BINTRAY_USER")
    bintrayKey = System.getenv("BINTRAY_KEY")
}

/// Set build configuration constants for use at runtime
buildConfig {
    useKotlinOutput { topLevelConstants = true }
    buildConfigField("String", "PLUGIN_VERSION", "\"${project.version}\"")
}

/// Add a task that allows us to print the current plugin version.
/// This is used in CI to validate the tag
tasks.register("printVersion") {
    doLast {
        println("${project.version}")
    }
}
