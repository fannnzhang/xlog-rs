import 'dart:ffi' as ffi;

import 'package:ffi/ffi.dart';

final class NativeLoggerHandle extends ffi.Opaque {}

@ffi.Native<ffi.Pointer<Utf8> Function()>(symbol: 'mxl_last_error_message')
external ffi.Pointer<Utf8> mxlLastErrorMessage();

@ffi.Native<ffi.Void Function(ffi.Pointer<Utf8>)>(symbol: 'mxl_string_free')
external void mxlStringFree(ffi.Pointer<Utf8> value);

@ffi.Native<
  ffi.Pointer<NativeLoggerHandle> Function(ffi.Pointer<Utf8>, ffi.Int32)
>(symbol: 'mxl_logger_new')
external ffi.Pointer<NativeLoggerHandle> mxlLoggerNew(
  ffi.Pointer<Utf8> configJson,
  int level,
);

@ffi.Native<ffi.Void Function(ffi.Pointer<NativeLoggerHandle>)>(
  symbol: 'mxl_logger_free',
)
external void mxlLoggerFree(ffi.Pointer<NativeLoggerHandle> handle);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Int32)>(
  symbol: 'mxl_logger_set_level',
)
external bool mxlLoggerSetLevel(
  ffi.Pointer<NativeLoggerHandle> handle,
  int level,
);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Int32)>(
  symbol: 'mxl_logger_set_appender_mode',
)
external bool mxlLoggerSetAppenderMode(
  ffi.Pointer<NativeLoggerHandle> handle,
  int mode,
);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Bool)>(
  symbol: 'mxl_logger_set_console_open',
)
external bool mxlLoggerSetConsoleOpen(
  ffi.Pointer<NativeLoggerHandle> handle,
  bool open,
);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Int64)>(
  symbol: 'mxl_logger_set_max_file_size',
)
external bool mxlLoggerSetMaxFileSize(
  ffi.Pointer<NativeLoggerHandle> handle,
  int maxBytes,
);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Int64)>(
  symbol: 'mxl_logger_set_max_alive_time',
)
external bool mxlLoggerSetMaxAliveTime(
  ffi.Pointer<NativeLoggerHandle> handle,
  int aliveSeconds,
);

@ffi.Native<ffi.Bool Function(ffi.Pointer<NativeLoggerHandle>, ffi.Bool)>(
  symbol: 'mxl_logger_flush',
)
external bool mxlLoggerFlush(ffi.Pointer<NativeLoggerHandle> handle, bool sync);

@ffi.Native<
  ffi.Bool Function(
    ffi.Pointer<NativeLoggerHandle>,
    ffi.Int32,
    ffi.Pointer<Utf8>,
    ffi.Pointer<Utf8>,
  )
>(symbol: 'mxl_logger_log')
external bool mxlLoggerLog(
  ffi.Pointer<NativeLoggerHandle> handle,
  int level,
  ffi.Pointer<Utf8> tag,
  ffi.Pointer<Utf8> message,
);

@ffi.Native<
  ffi.Bool Function(
    ffi.Pointer<NativeLoggerHandle>,
    ffi.Int32,
    ffi.Pointer<Utf8>,
    ffi.Pointer<Utf8>,
    ffi.Pointer<Utf8>,
    ffi.Uint32,
    ffi.Pointer<Utf8>,
  )
>(symbol: 'mxl_logger_log_with_meta')
external bool mxlLoggerLogWithMeta(
  ffi.Pointer<NativeLoggerHandle> handle,
  int level,
  ffi.Pointer<Utf8> tag,
  ffi.Pointer<Utf8> file,
  ffi.Pointer<Utf8> functionName,
  int line,
  ffi.Pointer<Utf8> message,
);

@ffi.Native<
  ffi.Pointer<Utf8> Function(ffi.Pointer<NativeLoggerHandle>, ffi.Uint32)
>(symbol: 'mxl_logger_list_files')
external ffi.Pointer<Utf8> mxlLoggerListFiles(
  ffi.Pointer<NativeLoggerHandle> handle,
  int limit,
);

@ffi.Native<ffi.Pointer<Utf8> Function(ffi.Pointer<NativeLoggerHandle>)>(
  symbol: 'mxl_logger_name_prefix',
)
external ffi.Pointer<Utf8> mxlLoggerNamePrefix(
  ffi.Pointer<NativeLoggerHandle> handle,
);

@ffi.Native<
  ffi.Pointer<Utf8> Function(
    ffi.Pointer<NativeLoggerHandle>,
    ffi.Int32,
    ffi.Pointer<Utf8>,
    ffi.Uint32,
    ffi.Uint32,
    ffi.Uint32,
  )
>(symbol: 'mxl_logger_benchmark')
external ffi.Pointer<Utf8> mxlLoggerBenchmark(
  ffi.Pointer<NativeLoggerHandle> handle,
  int level,
  ffi.Pointer<Utf8> tag,
  int iterations,
  int messageBytes,
  int threads,
);

@ffi.Native<ffi.Pointer<Utf8> Function(ffi.Pointer<Utf8>)>(
  symbol: 'mxl_decode_log_file',
)
external ffi.Pointer<Utf8> mxlDecodeLogFile(ffi.Pointer<Utf8> path);

@ffi.Native<ffi.Pointer<Utf8> Function()>(symbol: 'mxl_metrics_snapshot')
external ffi.Pointer<Utf8> mxlMetricsSnapshot();
