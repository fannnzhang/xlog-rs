import 'dart:io';

import 'package:xlog/xlog.dart';
import 'package:test/test.dart';

void main() {
  test(
    'initializes xlog, writes files, decodes logs, and reads metrics',
    () async {
      final tempRoot = await Directory.systemTemp.createTemp('xlog_test_');
      final logDir = Directory('${tempRoot.path}/logs')
        ..createSync(recursive: true);
      final cacheDir = Directory('${tempRoot.path}/cache')
        ..createSync(recursive: true);

      final logger = MarsXlogLogger.open(
        MarsXlogConfig(
          logDir: logDir.path,
          cacheDir: cacheDir.path,
          namePrefix: 'ffi_demo',
          appenderMode: MarsXlogAppenderMode.sync,
          enableConsole: false,
        ),
        level: MarsXlogLevel.debug,
      );

      addTearDown(() async {
        logger.dispose();
        if (await tempRoot.exists()) {
          await tempRoot.delete(recursive: true);
        }
      });

      logger.info('hello from dart ffi', tag: 'unit');
      logger.logWithMeta(
        MarsXlogLevel.warn,
        'with explicit metadata',
        tag: 'unit',
        file: 'unit_test.dart',
        functionName: 'main',
        line: 42,
      );
      logger.flush();

      final files = logger.listLogFiles(limit: 10);
      expect(files, isNotEmpty);

      final decoded = MarsXlog.decodeLogFile(files.first.path);
      expect(decoded, contains('hello from dart ffi'));
      expect(decoded, contains('with explicit metadata'));

      final metrics = MarsXlog.readMetricsSnapshot();
      expect(metrics, contains('xlog_'));

      final benchmark = await logger.runBenchmarkAsync(
        iterations: 128,
        threads: 2,
        messageBytes: 96,
        level: MarsXlogLevel.info,
        tag: 'bench',
      );
      expect(benchmark.iterations, 128);
      expect(benchmark.linesPerSecond, greaterThan(0));
    },
  );
}
