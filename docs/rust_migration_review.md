# Rust Migration Review & Parity Deep Dive

## 1. 结论总结

经过对 Rust 版实现（`xlog-core` / `xlog` / bindings）与 C++ 版（`mars/xlog`）的源码深度对比，Rust 核心库在以下层面与 C++ 实现了高度的语义对齐：
- **文件管理与滚动策略** (按天/大小切换、Cache 满溢移盘、并发写)。
- **Appender 异步/同步模式** (Mmap 预写、缓冲池阈值唤醒、压缩流水线)。
- **Crash-safe 恢复逻辑** (启动时 `recover_blocks`、torn-write 修补、Tip 标记)。
- **加密签名字节级协议** (ECDH-TEA 流水线加密逻辑、协议头构造)。

在最新一轮的深度对比中，我们进一步排查并修复了数个细微的边界差异：特别是发现在**后台 `flush` 与前端连续异步写入的交错场景**下，原生 Rust 使用的双锁架构（`RustBackend::async_state` 与 `AppenderEngine::state`）由于检查点时序问题，存在**极端条件下的数据丢失（Race Condition）**。

## 2. 本轮深层修复与功能对齐 (Deep Dive Fixes)

1. **[CRITICAL] 后台 Flush 导致的异步 Pending Block 截断丢数据问题**
   - **问题现象**：在异步高频写入时，如果恰好背景 Worker 线程达到 15 分钟超时或满 1/3 阈值触发 Flush，Worker 线程会抢占 Mmap 并刷盘。此时前端 `write_async_line` 如果恰好在检查 `async_flush_epoch` **之后**、写 Mmap **之前**，会导致前端状态机认为合并 Block 有效，最终用仅含后半段数据的 Block 覆盖了 Mmap（而此时 mmap 中旧的前半段数据已被 Worker 刷入磁盘）。结果：合并块的后半段数据丢失！
   - **C++ 对比**：C++ 版本在 `appender.cc:__WriteAsync` 和 `__AsyncLogThread` 中硬共用同一把 `mutex_buffer_async_` 大锁，因此压缩、追加、与后台刷盘在物理上完全互斥，无此竞争。
   - **解决方式**：不破坏 Rust 优良的无阻塞分治锁架构，通过在 `AppenderEngine::write_async_pending_check_epoch` 内部注入原子 Epoch 检验。前端在覆盖 Mmap 时若发现 Epoch 突变（已被刷盘），则直接摒弃已残缺的内存压缩块，重发原始文本进行安全重试（Loop Retry），彻底根除丢日志隐患。

## 3. 剩余细微未对齐项（待实现）

经过排查，当前仍有 2 项在 Wrapper 层级与 C++ 的 API 行为存在不匹配，主要涉及 JNI / FFI 层的 Raw 接口调用。

1. **`XLoggerInfo.traceLog` 旁路特性丢失**
   - **差异**：在 C++ `appender.cc` 中，对于 Android 平台，如果传入的 `XLoggerInfo::traceLog == 1`，即使 `consolelog_open_ == false` 也会强行输出到 Logcat。
   - **Rust 现状**：`mars-xlog-core::record::LogRecord` 尚未具有 `trace_log` 字段，且 `xlog` API 不支持旁路 Console 的独立判断。
2. **`XloggerWrite(instance_ptr==0)` 的全局 Raw Metadata 写路径**
   - **差异**：C++ 允许 JNI/FFI 层构建附带自定义 `pid` / `tid` / `maintid` 的 `XLoggerInfo` 并直接投递给 Global Appender。
   - **Rust 现状**：Rust 封装的 `write_with_meta` 会在本地以 `std::process::id()` 强制复写被传入的元数据，导致 Android 绑定层拿到的 Java Thread ID 被抹去。

---

由于上述 2 项系本轮比对的最后差异，我们已制定实施计划准备对其修复。
