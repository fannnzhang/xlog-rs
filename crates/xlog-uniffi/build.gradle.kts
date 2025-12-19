plugins {
    id("com.android.library") version "8.11.2"
    id("org.jetbrains.kotlin.android") version "2.2.0"
    kotlin("plugin.atomicfu") version "2.2.0"
    id("dev.gobley.cargo") version "0.3.7"
    id("dev.gobley.uniffi") version "0.3.7"
}

android {
    namespace = "com.tencent.mars.xlog"
    compileSdk = 35

    defaultConfig {
        minSdk = 24
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

    lint {
        abortOnError = false
    }
}

kotlin {
    jvmToolchain(21)
}

cargo {
    // Build the Rust package in this module directory.
    packageDirectory.set(layout.projectDirectory)
}

dependencies {
    // UniFFI plugin will add JNA/AtomicFU runtime deps if enabled (default).
}
