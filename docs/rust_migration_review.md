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

## 3. 本轮收口结果（已完成）

针对上一版中遗留的 Wrapper 级差异，已在当前分支完成收口：

1. **`traceLog` Android 旁路语义已补齐**
   - 在 `xlog` 新增 `RawLogMeta { trace_log }` 传递通道。
   - Rust backend 在 Android 上改为：`console_open == true` 或 `trace_log == true` 任一满足即写 Console，语义对齐 C++ `appender.cc`。
2. **全局 Raw Metadata 写路径与 PID/TID 复写策略已对齐**
   - 新增 `Xlog::appender_write_with_meta_raw(...)`，对应 `XloggerWrite(instance_ptr == 0, ...)` 能力。
   - `pid/tid/maintid` 填充规则按 C++ 双路径对齐：
     - `instance_ptr != 0`（Category 路径）：仅在三者全为 `-1` 时批量回填。
     - `instance_ptr == 0`（Global 路径）：逐字段按 `-1` 回填。
   - 由此避免 Java/JNI 侧传入线程元数据被 Rust 层强制覆写的问题。
3. **UniFFI / Harmony NAPI 接口覆盖已补齐**
   - 补齐实例控制面：`is_enabled/level/set_level/set_appender_mode/flush/set_console_log_open/set_max_file_size/set_max_alive_time`。
   - 补齐写入面：`log_with_meta/log_with_raw_meta` 与全局 `appender_write_with_raw_meta`。
   - 补齐工具面：`open_appender/close_appender/flush_all/current_log_path/current_log_cache_path/filepaths_from_timespan/make_logfile_name/oneshot_flush/dump/memory_dump`。
   - 绑定层当前能力面已对齐 `mars-xlog` Rust API。

上述修复后，本轮 review 中定义的 Rust 重构语义差异已全部收口。

## 4. 发布就绪度（截至 2026-03-04）

- `mars-xlog-core`：`cargo publish --dry-run` 通过。
- `mars-xlog`：依赖 `mars-xlog-core` 先发布到 crates.io（当前 dry-run 因索引无该包失败）。
- `mars-xlog-uniffi` / `oh-xlog`：依赖 `mars-xlog` 先发布到 crates.io（当前 dry-run 因索引无该包失败）。
- `mars-xlog-sys`：legacy FFI crate 的打包验证仍依赖仓库外路径（`third_party/mars`），需单独整改；不阻塞 Rust 主链路发布。

## 5. 性能优化深层 Review (2026-03-06)

针对当前 Rust 版本的性能表现（Sync 吞吐 34.9%，Async 吞吐 43.1%，p99 延迟较高），结合 `xlog-core` / `xlog` 与 C++ `mars/xlog` 当前实现，对上一轮“性能优化想法”做进一步筛选。结论不是“想法越多越好”，而是只保留真正符合当前瓶颈、且不破坏既有协议与恢复语义的实现项。

### 5.1 明确有价值，进入下一轮实现

1. **[AppenderEngine / Buffer] Async flush 路径继续压缩复制与清零**
   - 这条是高价值项。当前异步热路径虽然已经从“每条日志重建完整 pending block”收敛为增量 mmap 更新，但 `flush_pending_locked` 仍会把整个已用 mmap 区间复制到新 `Vec`，随后再对整个缓冲区执行清空。这会直接放大 async 的长尾延迟。
   - 下一步应继续把 flush 路径改成“优先复用 mmap 已有字节视图，只在 pending block 缺尾标记时做最小补齐”，并把 clear 从“整段 mmap 清零”收敛为“仅处理已用区间”。
   - 这条优化不改协议、不改恢复规则，只减少热路径内存复制和写放大。

2. **[FileManager] 按目录/按天缓存 append target，减少热路径目录扫描**
   - 这条也是高价值项。当前 `append_log_bytes` 进入 `select_append_path -> make_path_for_time -> next_file_index` 时，仍然会在边界条件下触发 `fs::read_dir`、`fs::metadata` 和路径重新选择。
   - C++ 路径的核心优势不是“逻辑不同”，而是 `logfile_` 和当天文件状态是长寿命的；Rust 下一步应补齐按目录/按天的 append target cache，把 steady-state 写入从“每次重新探测”改成“跨天、超 size、写失败时才失效重建”。
   - 这条优化直接针对 sync 仍明显落后 C++ 的主因，优先级高于继续做零碎的 formatter 微调。

3. **[Buffer / Crypto] 重写后的值钱部分：收敛 finalize/flush 阶段的额外复制**
   - 原始提法里把重点放在 `crypt_tail` 上，这个判断不够准确。`crypt_tail` 复制确实存在，但它最多只覆盖零碎尾字节，并不是当前吞吐和 p99 的主矛盾。
   - 真正值得继续做的是：flush/finalize 过程中避免把整段 pending 内容再次拼装成新 `Vec`，以及继续收敛 `buffer` 在废弃/回收场景下的零填充范围。
   - 这条仍属于“高价值，但优先级略低于前两项”。

### 5.2 有一定价值，但要先做 profiling 再决定

1. **[Atomic / Lock-free] 细化部分计数器到锁外**
   - 方向本身没问题，但不能把它误判成当前主瓶颈。`EngineState` 的主要成本并不在单个计数器自增，而在 buffer 一致性、flush 生命周期和文件 IO 这几个必须串行的关键区。
   - 可以在前两项落地后，再通过 profiling 判断 `async_pending_updates_since_persist`、`pending_async_flush` 等状态是否值得进一步拆成独立 `Atomic`。

2. **[SIMD] TEA / 压缩指令级加速**
   - 这条只在特定配置下才可能明显收益，例如启用了 crypt，或者压缩路径确实在 profile 中占主导。
   - 在当前阶段，先没有证据表明它比减少内存复制、减少目录扫描更值钱，因此只保留为实验项，不进入当前主线。

### 5.3 当前价值不高，暂不进入主线

1. **[Zero-Copy / Interning] 把 tag / filename / func 做成更激进的驻留或全局字符串**
   - 这条当前不建议继续投入。formatter 热路径已经改成 borrowed fields + 复用 scratch string，收益最大的那一层已经拿到了。
   - 继续做 string interning 或“完全 zero-copy 格式化”，实现复杂度高，但很难超过前两项带来的收益。

2. **[madvise / msync(MS_ASYNC)] mmap OS 指令级调优**
   - 这条风险高于收益。当前 mmap flush 仍然承担 crash-recovery 语义，贸然改为 `MS_ASYNC` 或加入激进 `madvise`，可能影响跨平台恢复行为。
   - 在没有专门 crash/断电验证前，不应进入主线。

3. **[sendfile / fcopyfile] Cache 文件零拷贝搬运**
   - 这条是冷路径优化，不是当前 benchmark 主瓶颈。`append_file_to_file` 只在 cache 文件搬运时触发，而不是每条日志热写路径。
   - 后续可以作为平台增强项单独评估，但不应抢占 async p99 和 sync 热路径的优化优先级。

4. **[线程优先级] setpriority / pthread 调度**
   - 这条平台相关性太强，而且会引入额外运维与权限复杂度。
   - 在当前还没有证明 Worker 线程被调度饿死之前，不进入主线。

### 5.4 本轮结论

下一轮主线只做两件事：

1. AppenderEngine / PersistentBuffer：去掉 async flush 路径的整段 `take_all + clear`。
2. FileManager：引入按目录/按天的 append target cache，减少 `read_dir / metadata / path-select`。

其余优化想法保留在文档中，但默认视为“实验项”或“后置项”，不再并行扩散实现范围。
