package com.tencent.mars.xlog.example

import android.os.Bundle
import android.text.method.ScrollingMovementMethod
import android.util.Log
import android.widget.Button
import android.widget.TextView
import androidx.appcompat.app.AppCompatActivity
import java.io.File
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

class MainActivity : AppCompatActivity() {
    private lateinit var outputView: TextView
    private var loggerHandle: Long = 0
    private var getLoggerHandle: Long = 0
    private val namePrefix = "xlog-demo"

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        outputView = findViewById(R.id.outputView)
        outputView.movementMethod = ScrollingMovementMethod()

        findViewById<Button>(R.id.btnRunAll).setOnClickListener { runAll() }
        findViewById<Button>(R.id.btnInit).setOnClickListener { initLogger() }
        findViewById<Button>(R.id.btnWrite).setOnClickListener { writeLogs() }
        findViewById<Button>(R.id.btnFlush).setOnClickListener { flushLogs() }
        findViewById<Button>(R.id.btnPaths).setOnClickListener { showPaths() }
        findViewById<Button>(R.id.btnDump).setOnClickListener { dumpBuffers() }
        findViewById<Button>(R.id.btnOneshot).setOnClickListener { oneshotFlush() }
        findViewById<Button>(R.id.btnClose).setOnClickListener { closeLogger() }
        findViewById<Button>(R.id.btnClear).setOnClickListener { clearOutput() }

        appendLine("Xlog JNI example ready.")
    }

    private fun initLogger() {
        ensureLogger()
    }

    private fun ensureLogger(): Long {
        if (loggerHandle != 0L) {
            return loggerHandle
        }

        val logDir = File(filesDir, "xlog").apply { mkdirs() }
        val cacheDir = File(cacheDir, "xlog_cache").apply { mkdirs() }

        val opened = XlogBridge.nativeOpenAppender(
            logDir.absolutePath,
            namePrefix,
            null,
            cacheDir.absolutePath,
            7,
            XlogBridge.AppenderMode.ASYNC.value,
            XlogBridge.CompressMode.ZLIB.value,
            6,
            XlogBridge.LogLevel.INFO.value,
        )

        loggerHandle = XlogBridge.nativeCreateLogger(
            logDir.absolutePath,
            namePrefix,
            null,
            cacheDir.absolutePath,
            7,
            XlogBridge.AppenderMode.ASYNC.value,
            XlogBridge.CompressMode.ZLIB.value,
            6,
            XlogBridge.LogLevel.INFO.value,
        )

        appendLine("open appender: $opened")
        appendLine("logger handle: $loggerHandle")
        return loggerHandle
    }

    private fun writeLogs() {
        val handle = ensureLogger()
        if (handle == 0L) {
            appendLine("init failed")
            return
        }
        XlogBridge.nativeSetConsoleLogOpen(handle, true)
        XlogBridge.nativeWrite(handle, XlogBridge.LogLevel.INFO.value, "demo", "hello from kotlin")
        XlogBridge.nativeWrite(handle, XlogBridge.LogLevel.DEBUG.value, "demo", "debug message")
        XlogBridge.nativeWriteWithMeta(
            handle,
            XlogBridge.LogLevel.WARN.value,
            "demo",
            "MainActivity.kt",
            "writeLogs",
            80,
            "warning with meta",
        )
        appendLine("write logs done")
    }

    private fun flushLogs() {
        val handle = ensureLogger()
        if (handle == 0L) {
            appendLine("init failed")
            return
        }
        XlogBridge.nativeFlush(handle, true)
        XlogBridge.nativeFlushAll(true)
        appendLine("flush done")
    }

    private fun showPaths() {
        val handle = ensureLogger()
        if (handle == 0L) {
            appendLine("init failed")
            return
        }
        val logPath = XlogBridge.nativeCurrentLogPath()
        val cachePath = XlogBridge.nativeCurrentLogCachePath()
        appendLine("current log path: $logPath")
        appendLine("current cache path: $cachePath")

        val timespan = SimpleDateFormat("yyyyMMdd", Locale.CHINA).format(Date()).toInt()
        val files = XlogBridge.nativeFilepathsFromTimespan(timespan, namePrefix)
        val names = XlogBridge.nativeMakeLogfileName(timespan, namePrefix)
        appendLine("filepaths from timespan($timespan): ${files.joinToString()}")
        appendLine("logfile names($timespan): ${names.joinToString()}")
    }

    private fun dumpBuffers() {
        val bytes = byteArrayOf(0x01, 0x02, 0x03, 0x04)
        val dump = XlogBridge.nativeDump(bytes)
        val memDump = XlogBridge.nativeMemoryDump(bytes)
        appendLine("dump: $dump")
        appendLine("memory dump: $memDump")
    }

    private fun oneshotFlush() {
        val logDir = File(filesDir, "xlog").apply { mkdirs() }
        val cacheDir = File(cacheDir, "xlog_cache").apply { mkdirs() }
        val result = XlogBridge.nativeOneshotFlush(
            logDir.absolutePath,
            namePrefix,
            null,
            cacheDir.absolutePath,
            7,
            XlogBridge.AppenderMode.ASYNC.value,
            XlogBridge.CompressMode.ZLIB.value,
            6,
        )
        val action = XlogBridge.FileIoAction.values().firstOrNull { it.value == result }
        appendLine("oneshot flush: ${action ?: result}")
    }

    private fun closeLogger() {
        if (loggerHandle != 0L) {
            val released = XlogBridge.nativeReleaseLogger(loggerHandle)
            appendLine("release logger: $released")
            loggerHandle = 0
        }
        if (getLoggerHandle != 0L && getLoggerHandle != loggerHandle) {
            val released = XlogBridge.nativeReleaseLogger(getLoggerHandle)
            appendLine("release get logger: $released")
            getLoggerHandle = 0
        }
        XlogBridge.nativeCloseAppender()
        appendLine("appender closed")
    }

    private fun runAll() {
        appendLine("== run all ==")
        val handle = ensureLogger()
        if (handle == 0L) {
            appendLine("init failed")
            return
        }

        XlogBridge.nativeSetConsoleLogOpen(handle, true)
        XlogBridge.nativeSetMaxFileSize(handle, 256 * 1024)
        XlogBridge.nativeSetMaxAliveTime(handle, 24 * 60 * 60)

        val enabledInfo = XlogBridge.nativeIsEnabled(handle, XlogBridge.LogLevel.INFO.value)
        appendLine("is enabled INFO: $enabledInfo")

        val beforeLevel = XlogBridge.nativeGetLevel(handle)
        appendLine("current level: $beforeLevel")
        XlogBridge.nativeSetLevel(handle, XlogBridge.LogLevel.WARN.value)
        val afterLevel = XlogBridge.nativeGetLevel(handle)
        appendLine("level after set: $afterLevel")

        XlogBridge.nativeSetAppenderMode(handle, XlogBridge.AppenderMode.SYNC.value)
        writeLogs()
        flushLogs()
        showPaths()
        dumpBuffers()
        oneshotFlush()

        if (getLoggerHandle == 0L) {
            getLoggerHandle = XlogBridge.nativeGetLogger(namePrefix)
        }
        val handle2 = getLoggerHandle
        appendLine("get logger by name: $handle2")
        if (handle2 != 0L) {
            XlogBridge.nativeWrite(handle2, XlogBridge.LogLevel.INFO.value, "demo", "logger from get()")
            appendLine("skip release: keep getLogger handle while main logger is active")
        }

        Log.i("XlogExample", "runAll done")
        appendLine("== run all done ==")
    }

    private fun appendLine(text: String) {
        Log.i("XlogExample", text)
        outputView.append(text)
        outputView.append("\n")
    }

    private fun clearOutput() {
        outputView.text = ""
    }
}
