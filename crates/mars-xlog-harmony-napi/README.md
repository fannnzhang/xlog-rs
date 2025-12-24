# oh-xlog

This crate provides Harmony/ohos N-API bindings via `ohrs`, wrapping the `mars-xlog`
Rust API for consumption in Harmony apps.

## build

Before building, set the `OHOS_NDK_HOME` environment variable to your HarmonyOS NDK path.

Build with:

```bash
ohrs build --arch=aarch -- -v
```
