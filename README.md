# mars-xlog (Rust workspace)

This workspace provides Rust bindings for Tencent Mars `xlog` using a C ABI wrapper.

## Crates
- `mars-xlog-sys`: raw FFI + native build (C/C++/ObjC++).
- `mars-xlog`: safe Rust wrapper API.
- `mars-xlog-uniffi`: minimal UniFFI surface (Kotlin/Swift friendly).
- `mars-xlog-android-jni`: JNI bridge used by the Android example app.
- `oh-xlog`: Harmony/ohos N-API bindings.

## Build notes
- Default source path: `./third_party/mars/mars` relative to this workspace.
- Override with `MARS_SRC_DIR=/path/to/mars` (the `mars` directory inside the Mars repo).
- Requires a C++14 compiler and `zlib`.
- iOS/macOS: links `Foundation` + `objc`.
- Android: links `log` + `android`.
- Harmony/ohos: links `hilog` (adjust if your toolchain differs).

## Mars submodule
This repository uses Tencent Mars as a git submodule at `third_party/mars`.
The build uses `third_party/mars/mars` (the Mars repo's `mars/` directory).

Initialize the submodule (first time):
```bash
git submodule update --init --recursive
```

Update the submodule to a newer commit:
```bash
git -C third_party/mars fetch
git -C third_party/mars checkout <tag-or-commit>
git add third_party/mars
```

## Example (Rust)
```rust
use mars_xlog::{AppenderMode, CompressMode, LogLevel, Xlog, XlogConfig};

fn main() -> anyhow::Result<()> {
    let cfg = XlogConfig::new("/tmp/xlog", "demo")
        .mode(AppenderMode::Async)
        .compress_mode(CompressMode::Zlib)
        .compress_level(6);

    let logger = Xlog::init(cfg, LogLevel::Debug)?;
    logger.log(LogLevel::Info, None, "hello from rust");
    logger.flush(true);
    Ok(())
}
```

## Example (tracing + tracing-subscriber)
Enable feature `tracing` and build an `XlogLayer`:
```rust
use mars_xlog::{LogLevel, Xlog, XlogConfig, XlogLayer, XlogLayerConfig};
use tracing_subscriber::prelude::*;

fn init_tracing() -> anyhow::Result<mars_xlog::XlogLayerHandle> {
    let cfg = XlogConfig::new("/tmp/xlog", "demo");
    let logger = Xlog::init(cfg, LogLevel::Info)?;

    let (layer, handle) = XlogLayer::with_config(
        logger,
        XlogLayerConfig::new(LogLevel::Info).enabled(true),
    );

    let subscriber = tracing_subscriber::registry().with(layer);
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(handle)
}
```

You can toggle the layer dynamically (mobile-friendly):
```rust
handle.set_enabled(false);
handle.set_level(LogLevel::Warn);
```

## Example (Android JNI)
An Android app example that calls the `mars-xlog` crate via JNI lives at:
`examples/android-jni`. See its README for build steps.

## Notes
 - `Xlog::log`/`Xlog::write` capture caller file/line but not function name. Use the `xlog!` macros (feature `macros`) or `write_with_meta` for full metadata.
- iOS ObjC++ sources are included to preserve original behavior.
 - For low-level/global appender APIs, use `mars-xlog-sys`.

## License
MIT. See `LICENSE` and `NOTICE`.
