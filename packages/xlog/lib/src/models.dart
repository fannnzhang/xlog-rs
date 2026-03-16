class MarsXlogException implements Exception {
  const MarsXlogException(this.message);

  final String message;

  @override
  String toString() => 'MarsXlogException: $message';
}

enum MarsXlogLevel {
  verbose,
  debug,
  info,
  warn,
  error,
  fatal,
  none;

  int get ffiValue => index;

  String get label => switch (this) {
    MarsXlogLevel.verbose => 'VERBOSE',
    MarsXlogLevel.debug => 'DEBUG',
    MarsXlogLevel.info => 'INFO',
    MarsXlogLevel.warn => 'WARN',
    MarsXlogLevel.error => 'ERROR',
    MarsXlogLevel.fatal => 'FATAL',
    MarsXlogLevel.none => 'NONE',
  };
}

enum MarsXlogAppenderMode {
  async,
  sync;

  int get ffiValue => index;

  String get label => switch (this) {
    MarsXlogAppenderMode.async => 'Async',
    MarsXlogAppenderMode.sync => 'Sync',
  };
}

enum MarsXlogCompressMode {
  zlib,
  zstd;

  int get ffiValue => index;

  String get label => switch (this) {
    MarsXlogCompressMode.zlib => 'Zlib',
    MarsXlogCompressMode.zstd => 'Zstd',
  };
}

class MarsXlogConfig {
  const MarsXlogConfig({
    required this.logDir,
    required this.namePrefix,
    this.pubKey,
    this.cacheDir,
    this.cacheDays = 0,
    this.appenderMode = MarsXlogAppenderMode.async,
    this.compressMode = MarsXlogCompressMode.zlib,
    this.compressLevel = 6,
    this.enableConsole = true,
    this.maxFileSizeBytes,
    this.maxAliveTimeSeconds,
  });

  final String logDir;
  final String namePrefix;
  final String? pubKey;
  final String? cacheDir;
  final int cacheDays;
  final MarsXlogAppenderMode appenderMode;
  final MarsXlogCompressMode compressMode;
  final int compressLevel;
  final bool enableConsole;
  final int? maxFileSizeBytes;
  final int? maxAliveTimeSeconds;

  Map<String, Object?> toJson() => <String, Object?>{
    'logDir': logDir,
    'namePrefix': namePrefix,
    'pubKey': pubKey,
    'cacheDir': cacheDir,
    'cacheDays': cacheDays,
    'mode': appenderMode.ffiValue,
    'compressMode': compressMode.ffiValue,
    'compressLevel': compressLevel,
    'enableConsole': enableConsole,
    'maxFileSizeBytes': maxFileSizeBytes,
    'maxAliveTimeSeconds': maxAliveTimeSeconds,
  };
}

class MarsXlogLogFile {
  const MarsXlogLogFile({
    required this.path,
    required this.fileName,
    required this.extension,
    required this.sizeBytes,
    required this.modifiedAt,
  });

  final String path;
  final String fileName;
  final String extension;
  final int sizeBytes;
  final DateTime modifiedAt;

  factory MarsXlogLogFile.fromJson(Map<String, Object?> json) {
    return MarsXlogLogFile(
      path: json['path']! as String,
      fileName: json['fileName']! as String,
      extension: json['extension']! as String,
      sizeBytes: (json['sizeBytes']! as num).toInt(),
      modifiedAt: DateTime.fromMillisecondsSinceEpoch(
        (json['modifiedAtMillis']! as num).toInt(),
      ),
    );
  }
}

class MarsXlogBenchmarkReport {
  const MarsXlogBenchmarkReport({
    required this.iterations,
    required this.threads,
    required this.messageBytes,
    required this.elapsedMicros,
    required this.linesPerSecond,
    required this.bytesPerSecond,
    required this.currentLogPath,
  });

  final int iterations;
  final int threads;
  final int messageBytes;
  final int elapsedMicros;
  final double linesPerSecond;
  final double bytesPerSecond;
  final String? currentLogPath;

  Duration get elapsed => Duration(microseconds: elapsedMicros);

  factory MarsXlogBenchmarkReport.fromJson(Map<String, Object?> json) {
    return MarsXlogBenchmarkReport(
      iterations: (json['iterations']! as num).toInt(),
      threads: (json['threads']! as num).toInt(),
      messageBytes: (json['messageBytes']! as num).toInt(),
      elapsedMicros: (json['elapsedMicros']! as num).toInt(),
      linesPerSecond: (json['linesPerSecond']! as num).toDouble(),
      bytesPerSecond: (json['bytesPerSecond']! as num).toDouble(),
      currentLogPath: json['currentLogPath'] as String?,
    );
  }
}
