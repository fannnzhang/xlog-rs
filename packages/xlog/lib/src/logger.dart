import 'dart:convert';
import 'dart:ffi' as ffi;
import 'dart:isolate';

import 'package:ffi/ffi.dart';

import 'xlog_bindings.dart' as bindings;
import 'models.dart';

MarsXlogBenchmarkReport _runBenchmarkInBackground(
  ({
    int address,
    int iterations,
    int threads,
    int messageBytes,
    int level,
    String? tag,
  })
  request,
) {
  return using((arena) {
    final handle = ffi.Pointer<bindings.NativeLoggerHandle>.fromAddress(
      request.address,
    );
    final raw = _takeStringResult(
      bindings.mxlLoggerBenchmark(
        handle,
        request.level,
        _toNativeUtf8OrNull(request.tag, arena),
        request.iterations,
        request.messageBytes,
        request.threads,
      ),
      'failed to run benchmark',
    );
    return MarsXlogBenchmarkReport.fromJson(
      _decodeObject(raw, 'invalid benchmark payload'),
    );
  });
}

ffi.Pointer<Utf8> _toNativeUtf8OrNull(String? value, Arena arena) {
  if (value == null || value.isEmpty) {
    return ffi.nullptr;
  }
  return value.toNativeUtf8(allocator: arena);
}

Never _throwLastError(String fallback) {
  final pointer = bindings.mxlLastErrorMessage();
  final message = pointer == ffi.nullptr ? fallback : pointer.toDartString();
  if (pointer != ffi.nullptr) {
    bindings.mxlStringFree(pointer);
  }
  throw MarsXlogException(message);
}

void _check(bool ok, String fallback) {
  if (!ok) {
    _throwLastError(fallback);
  }
}

String _takeStringResult(ffi.Pointer<Utf8> pointer, String fallback) {
  if (pointer == ffi.nullptr) {
    _throwLastError(fallback);
  }
  final value = pointer.toDartString();
  bindings.mxlStringFree(pointer);
  return value;
}

Map<String, Object?> _decodeObject(String value, String fallback) {
  final decoded = jsonDecode(value);
  if (decoded is! Map<String, Object?>) {
    throw MarsXlogException(fallback);
  }
  return decoded;
}

List<Object?> _decodeList(String value, String fallback) {
  final decoded = jsonDecode(value);
  if (decoded is! List<Object?>) {
    throw MarsXlogException(fallback);
  }
  return decoded;
}

final class MarsXlog {
  const MarsXlog._();

  static String readMetricsSnapshot() {
    final pointer = bindings.mxlMetricsSnapshot();
    return _takeStringResult(pointer, 'failed to read metrics snapshot');
  }

  static Future<String> readMetricsSnapshotAsync() {
    return Isolate.run(readMetricsSnapshot);
  }

  static String decodeLogFile(String path) {
    return using((arena) {
      final pointer = bindings.mxlDecodeLogFile(
        path.toNativeUtf8(allocator: arena),
      );
      return _takeStringResult(pointer, 'failed to decode log file');
    });
  }

  static Future<String> decodeLogFileAsync(String path) {
    return Isolate.run(() => decodeLogFile(path));
  }
}

final class MarsXlogLogger {
  MarsXlogLogger._(this.config, this._handle, {required bool owned})
    : _owned = owned;

  factory MarsXlogLogger.open(
    MarsXlogConfig config, {
    MarsXlogLevel level = MarsXlogLevel.info,
  }) {
    return using((arena) {
      final configJson = jsonEncode(
        config.toJson(),
      ).toNativeUtf8(allocator: arena);
      final handle = bindings.mxlLoggerNew(configJson, level.ffiValue);
      if (handle == ffi.nullptr) {
        _throwLastError('failed to create logger');
      }
      return MarsXlogLogger._(config, handle, owned: true);
    });
  }

  final MarsXlogConfig config;
  final ffi.Pointer<bindings.NativeLoggerHandle> _handle;
  final bool _owned;
  bool _disposed = false;

  bool get isDisposed => _disposed;

  int get nativeAddress => _handle.address;

  void _ensureOpen() {
    if (_disposed) {
      throw const MarsXlogException('logger has already been disposed');
    }
  }

  void dispose() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    if (_owned) {
      bindings.mxlLoggerFree(_handle);
    }
  }

  void setLevel(MarsXlogLevel level) {
    _ensureOpen();
    _check(
      bindings.mxlLoggerSetLevel(_handle, level.ffiValue),
      'failed to set log level',
    );
  }

  void setAppenderMode(MarsXlogAppenderMode mode) {
    _ensureOpen();
    _check(
      bindings.mxlLoggerSetAppenderMode(_handle, mode.ffiValue),
      'failed to set appender mode',
    );
  }

  void setConsoleOpen(bool open) {
    _ensureOpen();
    _check(
      bindings.mxlLoggerSetConsoleOpen(_handle, open),
      'failed to update console setting',
    );
  }

  void setMaxFileSize(int maxBytes) {
    _ensureOpen();
    _check(
      bindings.mxlLoggerSetMaxFileSize(_handle, maxBytes),
      'failed to set max file size',
    );
  }

  void setMaxAliveTime(int aliveSeconds) {
    _ensureOpen();
    _check(
      bindings.mxlLoggerSetMaxAliveTime(_handle, aliveSeconds),
      'failed to set max alive time',
    );
  }

  void flush({bool sync = true}) {
    _ensureOpen();
    _check(bindings.mxlLoggerFlush(_handle, sync), 'failed to flush logger');
  }

  String namePrefix() {
    _ensureOpen();
    return _takeStringResult(
      bindings.mxlLoggerNamePrefix(_handle),
      'failed to fetch name prefix',
    );
  }

  void log(MarsXlogLevel level, String message, {String? tag}) {
    _ensureOpen();
    using((arena) {
      _check(
        bindings.mxlLoggerLog(
          _handle,
          level.ffiValue,
          _toNativeUtf8OrNull(tag, arena),
          message.toNativeUtf8(allocator: arena),
        ),
        'failed to write log entry',
      );
    });
  }

  void logWithMeta(
    MarsXlogLevel level,
    String message, {
    String? tag,
    String? file,
    String? functionName,
    int line = 0,
  }) {
    _ensureOpen();
    using((arena) {
      _check(
        bindings.mxlLoggerLogWithMeta(
          _handle,
          level.ffiValue,
          _toNativeUtf8OrNull(tag, arena),
          _toNativeUtf8OrNull(file, arena),
          _toNativeUtf8OrNull(functionName, arena),
          line,
          message.toNativeUtf8(allocator: arena),
        ),
        'failed to write log entry with metadata',
      );
    });
  }

  void verbose(String message, {String? tag}) =>
      log(MarsXlogLevel.verbose, message, tag: tag);

  void debug(String message, {String? tag}) =>
      log(MarsXlogLevel.debug, message, tag: tag);

  void info(String message, {String? tag}) =>
      log(MarsXlogLevel.info, message, tag: tag);

  void warn(String message, {String? tag}) =>
      log(MarsXlogLevel.warn, message, tag: tag);

  void error(String message, {String? tag}) =>
      log(MarsXlogLevel.error, message, tag: tag);

  List<MarsXlogLogFile> listLogFiles({int limit = 50}) {
    _ensureOpen();
    final raw = _takeStringResult(
      bindings.mxlLoggerListFiles(_handle, limit),
      'failed to list log files',
    );
    final items = _decodeList(raw, 'invalid log file payload');
    return items
        .map(
          (entry) => MarsXlogLogFile.fromJson(entry! as Map<String, Object?>),
        )
        .toList(growable: false);
  }

  MarsXlogBenchmarkReport runBenchmark({
    int iterations = 10000,
    int threads = 4,
    int messageBytes = 160,
    MarsXlogLevel level = MarsXlogLevel.info,
    String? tag,
  }) {
    _ensureOpen();
    return using((arena) {
      final raw = _takeStringResult(
        bindings.mxlLoggerBenchmark(
          _handle,
          level.ffiValue,
          _toNativeUtf8OrNull(tag, arena),
          iterations,
          messageBytes,
          threads,
        ),
        'failed to run benchmark',
      );
      return MarsXlogBenchmarkReport.fromJson(
        _decodeObject(raw, 'invalid benchmark payload'),
      );
    });
  }

  Future<MarsXlogBenchmarkReport> runBenchmarkAsync({
    int iterations = 10000,
    int threads = 4,
    int messageBytes = 160,
    MarsXlogLevel level = MarsXlogLevel.info,
    String? tag,
  }) {
    _ensureOpen();
    return Isolate.run(
      () => _runBenchmarkInBackground((
        address: nativeAddress,
        iterations: iterations,
        threads: threads,
        messageBytes: messageBytes,
        level: level.ffiValue,
        tag: tag,
      )),
    );
  }
}
