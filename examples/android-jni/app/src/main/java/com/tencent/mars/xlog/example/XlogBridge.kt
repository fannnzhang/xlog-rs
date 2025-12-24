package com.tencent.mars.xlog.example

object XlogBridge {
    init {
        System.loadLibrary("mars_xlog_android_jni")
    }

    enum class LogLevel(val value: Int) {
        VERBOSE(0),
        DEBUG(1),
        INFO(2),
        WARN(3),
        ERROR(4),
        FATAL(5),
        NONE(6),
    }

    enum class AppenderMode(val value: Int) {
        ASYNC(0),
        SYNC(1),
    }

    enum class CompressMode(val value: Int) {
        ZLIB(0),
        ZSTD(1),
    }

    enum class ConsoleFun(val value: Int) {
        PRINTF(0),
        NSLOG(1),
        OSLOG(2),
    }

    enum class FileIoAction(val value: Int) {
        NONE(0),
        SUCCESS(1),
        UNNECESSARY(2),
        OPEN_FAILED(3),
        READ_FAILED(4),
        WRITE_FAILED(5),
        CLOSE_FAILED(6),
        REMOVE_FAILED(7),
        ERROR(-1),
    }

    external fun nativeCreateLogger(
        logDir: String,
        namePrefix: String,
        pubKey: String?,
        cacheDir: String?,
        cacheDays: Int,
        mode: Int,
        compressMode: Int,
        compressLevel: Int,
        level: Int,
    ): Long

    external fun nativeGetLogger(namePrefix: String): Long
    external fun nativeReleaseLogger(handle: Long): Boolean

    external fun nativeOpenAppender(
        logDir: String,
        namePrefix: String,
        pubKey: String?,
        cacheDir: String?,
        cacheDays: Int,
        mode: Int,
        compressMode: Int,
        compressLevel: Int,
        level: Int,
    ): Boolean

    external fun nativeCloseAppender()
    external fun nativeFlushAll(sync: Boolean)
    external fun nativeIsEnabled(handle: Long, level: Int): Boolean
    external fun nativeGetLevel(handle: Long): Int
    external fun nativeSetLevel(handle: Long, level: Int)
    external fun nativeSetAppenderMode(handle: Long, mode: Int)
    external fun nativeFlush(handle: Long, sync: Boolean)
    external fun nativeSetConsoleLogOpen(handle: Long, open: Boolean)
    external fun nativeSetMaxFileSize(handle: Long, maxBytes: Long)
    external fun nativeSetMaxAliveTime(handle: Long, aliveSeconds: Long)

    external fun nativeWrite(handle: Long, level: Int, tag: String?, message: String)
    external fun nativeWriteWithMeta(
        handle: Long,
        level: Int,
        tag: String?,
        file: String,
        func: String,
        line: Int,
        message: String,
    )

    external fun nativeCurrentLogPath(): String?
    external fun nativeCurrentLogCachePath(): String?
    external fun nativeFilepathsFromTimespan(timespan: Int, prefix: String): Array<String>
    external fun nativeMakeLogfileName(timespan: Int, prefix: String): Array<String>

    external fun nativeOneshotFlush(
        logDir: String,
        namePrefix: String,
        pubKey: String?,
        cacheDir: String?,
        cacheDays: Int,
        mode: Int,
        compressMode: Int,
        compressLevel: Int,
    ): Int

    external fun nativeDump(buffer: ByteArray): String
    external fun nativeMemoryDump(buffer: ByteArray): String
}
