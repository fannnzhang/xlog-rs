# mars-xlog (Rust workspace)

This workspace provides Rust bindings for Tencent Mars `xlog` using a C ABI wrapper.

## Crates
- `mars-xlog-sys`: raw FFI + native build (C/C++/ObjC++).
- `mars-xlog`: safe Rust wrapper API.
- `mars-xlog-uniffi`: minimal UniFFI surface (Kotlin/Swift friendly).

## Build notes
- Default source path: `./third_party/mars/mars` relative to this workspace.
- Override with `MARS_SRC_DIR=/path/to/mars` (the `mars` directory inside the Mars repo).
- Requires a C++14 compiler and `zlib`.
- iOS/macOS: links `Foundation` + `objc`.
- Android: links `log` + `android`.
- Harmony/ohos: links `hilog` (adjust if your toolchain differs).

## Mars subtree
This repository vendors Tencent Mars as a git subtree at `third_party/mars`.
The build uses `third_party/mars/mars` (the Mars repo's `mars/` directory).

Update the subtree:
```bash
git subtree pull --prefix third_party/mars https://github.com/Tencent/mars.git master --squash
```

Add the subtree (first time):
```bash
git subtree add --prefix third_party/mars https://github.com/Tencent/mars.git master --squash
```

## Example (Rust)
```rust
use mars_xlog::{AppenderMode, CompressMode, LogLevel, Xlog, XlogConfig};

fn main() -> anyhow::Result<()> {
    let cfg = XlogConfig::new("/tmp/xlog", "demo")
        .mode(AppenderMode::Async)
        .compress_mode(CompressMode::Zlib)
        .compress_level(6);

    let logger = Xlog::new(cfg, LogLevel::Debug)?;
    logger.write(LogLevel::Info, Some("demo"), "hello from rust");
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
    let logger = Xlog::new(cfg, LogLevel::Info)?;

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

## Notes
- `Xlog::write` does not capture caller file/line. Use the `xlog!` macros (feature `macros`) or `write_with_meta` for accurate metadata.
- iOS ObjC++ sources are included to preserve original behavior.

## License
MIT. See `LICENSE` and `NOTICE`.
