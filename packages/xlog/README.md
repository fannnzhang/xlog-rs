# xlog

Flutter FFI package for `mars-xlog`, built on top of Dart native assets and a Rust `cdylib`.

This package is intentionally implemented as a `package_ffi`-style native-assets package instead of the older `plugin_ffi` pattern, because Flutter's current recommended path for direct FFI distribution is native assets.

## What it includes

- Direct logger lifecycle and write APIs for Flutter/Dart.
- Native benchmark entrypoint that runs on the Rust side.
- Log artifact listing plus xlog file decoding for local inspection.
- Prometheus metrics snapshot export for runtime observability.
- A full Flutter example app that acts as a diagnostics console.

## Requirements

- Flutter via `fvm`, pinned here to `3.41.4`.
- Dart `3.11.1` or newer.
- `rustup` with the targets listed in [`rust/rust-toolchain.toml`](./rust/rust-toolchain.toml).

## Quick start

```bash
cd packages/xlog
fvm flutter pub get
fvm flutter test
```

Run the example:

```bash
cd packages/xlog/example
fvm flutter run -d macos
```

## Usage

```dart
import 'package:xlog/xlog.dart';

final logger = MarsXlogLogger.open(
  MarsXlogConfig(
    logDir: '/tmp/xlog',
    cacheDir: '/tmp/xlog/cache',
    namePrefix: 'flutter_demo',
    appenderMode: MarsXlogAppenderMode.async,
    compressMode: MarsXlogCompressMode.zstd,
  ),
  level: MarsXlogLevel.debug,
);

logger.info('hello from flutter', tag: 'demo');
logger.logWithMeta(
  MarsXlogLevel.warn,
  'explicit metadata',
  file: 'lib/main.dart',
  functionName: 'main',
  line: 12,
);
logger.flush();

final files = logger.listLogFiles();
final decoded = MarsXlog.decodeLogFile(files.first.path);
final metrics = MarsXlog.readMetricsSnapshot();

logger.dispose();
```

## Structure

- `lib/`: public Dart API and `@Native` bindings.
- `hook/build.dart`: native-assets hook using `native_toolchain_rust`.
- `rust/`: Rust `cdylib` that bridges Dart FFI to `mars-xlog`.
- `example/`: diagnostics UI for real scenarios, stress tests, file browsing, and metrics.

## Notes

- `MarsXlog.decodeLogFile` can decode plaintext xlog blocks. If a file was written with encryption enabled, encrypted blocks are reported but cannot be decrypted without the matching private key.
- Benchmarks are executed inside the Rust library so the numbers focus on xlog write throughput rather than Dart-to-native crossing overhead.
