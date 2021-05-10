import org.jetbrains.kotlin.gradle.tasks.KotlinCompile

/// The plugin version number – change this to match whatever your tag will be
version = "0.6.0"
group = "com.automattic.android"

buildscript {
    repositories {
        maven { url = uri("https://a8c-libs.s3.amazonaws.com/android") }
    }
    dependencies {
        classpath("com.automattic.android:publish-to-s3:0.3")
    }
}

plugins {
    `kotlin-dsl`
    id("com.github.gmazzo.buildconfig") version "2.0.2"
}

apply(plugin = "com.automattic.android.publish-plugin-to-s3")

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
