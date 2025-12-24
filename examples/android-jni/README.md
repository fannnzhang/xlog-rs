# Android JNI example (mars-xlog)

This app calls the `mars-xlog` Rust crate from Kotlin through JNI and exercises most APIs.

## Prerequisites
- Android SDK + NDK 23.1.7779620
- JDK 21
- Rust targets:
  - `aarch64-linux-android`
  - `armv7-linux-androideabi`

## Build (Android Studio)
1. Open this directory (`examples/android-jni`) in Android Studio.
2. Ensure `ANDROID_HOME` or `ANDROID_SDK_ROOT` is set and the NDK version matches.
3. Build/Run the `app` configuration.

The build runs `ANDROID_NDK_HOME="$ANDROID_HOME/ndk/23.1.7779620" cargo ndk -t arm64-v8a -t armeabi-v7a -o app/src/main/jniLibs build -p mars-xlog-android-jni --platform 21 --release`.

## Build from CLI
From `examples/android-jni`:
```bash
./gradlew :app:assembleDebug
```

Optional release build:
```bash
./gradlew :app:assembleRelease -PrustProfile=release
```

## What it tests
- Create/get/release loggers
- Appender open/close
- Write logs + write with meta
- Level + enabled checks
- Appender mode switch
- Flush/flushAll
- Console log open
- Max file size / alive time
- Current log paths
- Timespan file lookup
- Oneshot flush
- Dump/memory dump helpers

Outputs are visible in the in-app panel and logcat (`XlogExample`).
