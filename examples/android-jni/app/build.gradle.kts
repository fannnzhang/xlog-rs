import java.io.File
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.tencent.mars.xlog.example"
    compileSdk = 35
    ndkVersion = "23.1.7779620"

    defaultConfig {
        applicationId = "com.tencent.mars.xlog.example"
        minSdk = 24
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"

        ndk {
            abiFilters += listOf("armeabi-v7a", "arm64-v8a")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    kotlinOptions {
        jvmTarget = "21"
    }

    packaging {
        jniLibs.keepDebugSymbols += "**/*.so"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }
}

fun loadLocalProperties(projectRoot: File): Properties {
    val props = Properties()
    val file = File(projectRoot, "local.properties")
    if (file.exists()) {
        file.inputStream().use { props.load(it) }
    }
    return props
}

fun resolveCargoExecutable(projectRoot: File): String {
    val localProps = loadLocalProperties(projectRoot)
    val fromLocal = localProps.getProperty("cargo.path")
    if (!fromLocal.isNullOrBlank()) {
        return fromLocal
    }

    val fromProp = (project.findProperty("cargoPath") as String?)
    if (!fromProp.isNullOrBlank()) {
        return fromProp
    }

    val cargoHome = System.getenv("CARGO_HOME")
    if (!cargoHome.isNullOrBlank()) {
        val cargo = File(cargoHome, "bin/cargo")
        if (cargo.exists()) {
            return cargo.absolutePath
        }
    }

    val home = System.getenv("HOME")
    if (!home.isNullOrBlank()) {
        val cargo = File(home, ".cargo/bin/cargo")
        if (cargo.exists()) {
            return cargo.absolutePath
        }
    }

    return "cargo"
}

androidComponents {
    onVariants { variant ->
        val variantName = variant.name
        val capName = variantName.replaceFirstChar { if (it.isLowerCase()) it.titlecase() else it.toString() }

        val cargoTask = tasks.register<Exec>("cargoNdkBuild$capName") {
            group = "build"
            description = "Build Rust JNI libs via cargo-ndk for $variantName."

            val repoRoot = rootDir.parentFile?.parentFile ?: rootDir
            val outputDir = File(rootDir, "app/src/main/jniLibs")

            workingDir = repoRoot
            outputs.dir(outputDir)
            outputs.upToDateWhen { false }

            doFirst {
                outputDir.mkdirs()

                val localProps = loadLocalProperties(rootDir)
                val sdkDir = localProps.getProperty("sdk.dir")
                    ?: System.getenv("ANDROID_HOME")
                    ?: System.getenv("ANDROID_SDK_ROOT")

                val ndkVersion = "23.1.7779620"
                val ndkHome = System.getenv("ANDROID_NDK_HOME")
                    ?: sdkDir?.let { File(it, "ndk/$ndkVersion").absolutePath }

                if (ndkHome.isNullOrBlank()) {
                    throw GradleException("ANDROID_NDK_HOME not set and SDK dir not found in local.properties.")
                }

                environment("ANDROID_NDK_HOME", ndkHome)

                val cargoExe = resolveCargoExecutable(rootDir)
                val cargoDir = File(cargoExe).parentFile

                if (cargoDir != null && cargoDir.exists()) {
                    val currentPath = System.getenv("PATH") ?: ""
                    if (!currentPath.split(File.pathSeparator).contains(cargoDir.absolutePath)) {
                        environment("PATH", cargoDir.absolutePath + File.pathSeparator + currentPath)
                    }
                }

                val args = mutableListOf(
                    cargoExe,
                    "ndk",
                    "-t",
                    "arm64-v8a",
                    "-t",
                    "armeabi-v7a",
                    "-o",
                    outputDir.absolutePath,
                    "build",
                    "-p",
                    "mars-xlog-android-jni",
                    "--platform",
                    "21",
                )
                if (variant.buildType == "release") {
                    args.add("--release")
                }

                commandLine(args)
            }
        }

        afterEvaluate {
            tasks.named("pre${capName}Build") {
                dependsOn(cargoTask)
            }
        }
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("com.google.android.material:material:1.12.0")
}
