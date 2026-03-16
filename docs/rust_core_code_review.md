# mars-xlog-core 代码架构与可维护性审查

> 审查日期: 2026-03-16
> 复核日期: 2026-03-16
> 审查范围: `crates/xlog-core/src/`
> 审查维度: 架构分层、代码重复、工程质量、类型设计、测试覆盖

## 0. 当前结论

这份 review 里有几条在当前代码下已经不成立，不能保留：

1. `oneshot_flush` 缺少端到端测试，这条不成立；当前已有 `crates/xlog-core/tests/oneshot_flush.rs`
2. `AppenderEngine` 缺少 async/sync 集成测试，这条不成立；当前已有 `crates/xlog-core/tests/async_engine.rs`
3. `FileManager` 仍是当前 correctness active blocker，这条不成立；单写者约束已通过 `log_dir/cache_dir` 锁、README 和回归测试显式化
4. `AppenderEngine` 在高并发 sync 路径上统一争抢 `EngineState` 锁，这条不成立；sync `write_block()` 不进入 `EngineState` mutex

保留并成立的重点有：

1. recovery helper 在 `appender_engine.rs` 与 `oneshot.rs` 之间存在重复实现
2. `FileManager` 复杂度偏高，后续仍值得拆分
3. mutex poison 处理在少数 setter 路径上存在不一致，可能导致 state 副本与 atomic 副本分裂
4. 若干 magic number / 临时分配 / 边界转换可以收口
5. `compress.rs` 缺少 `ZlibStreamCompressor` roundtrip 测试

## 1. 本轮已处理

### 1.1 去重 recovery helper

已将以下函数提取到独立内部模块 `crates/xlog-core/src/recovery.rs`：

1. `magic_profile`
2. `build_sync_tip_block`
3. `current_mark_info`

`appender_engine.rs` 与 `oneshot.rs` 现在复用同一实现。

### 1.2 收口部分工程性细节

本轮已同步处理：

1. `set_max_file_size` / `set_max_alive_time` 在 mutex poison 时改为 fast-fail，避免 atomic 与 `EngineState` 副本不一致
2. `protocol.rs` 增加 header 字段偏移常量，并移除 `buffer.rs` 里的长度字段 magic number
3. `oneshot.rs` 的 `u64 -> usize` 长度转换改为 `usize::try_from`
4. `file_manager.rs` 使用 `LOG_EXT_WITH_DOT`，消除热路径上的重复 `format!(".{LOG_EXT}")`
5. `mmap_store.rs` 的零填充分配改为固定缓冲区分块写入
6. `platform_console` 的短标签复用 `record::LogLevel::short()`

### 1.3 补测试

本轮新增或保留有效测试关注点：

1. recovery helper 行为测试集中到 `recovery.rs`
2. `ZlibStreamCompressor` roundtrip 测试已补充

## 2. 仍然成立的技术债

### 2.1 FileManager 复杂度偏高

`file_manager.rs` 仍同时承担：

1. 路径解析与日期编码
2. append target 缓存
3. active file buffered I/O
4. cache/log 路由
5. cache 迁移与过期清理
6. 进程级单写者锁

这会持续抬高 `append_log_slices_inner` 一类核心逻辑的理解和测试成本。

建议后续拆分方向：

1. path/date resolver
2. active append writer
3. cache/log append routing

### 2.2 ConsoleLevel 与 LogLevel 仍是两套枚举

这条“完全冗余”不应表述得过强，但问题仍存在：

1. `platform_console::ConsoleLevel`
2. `record::LogLevel`

两者语义相近，维护上容易漂移。当前已让 console 的短标签复用 `LogLevel::short()`，但类型本身仍未统一。

### 2.3 压缩器内存保留策略仍可优化

原审查里“无限增长”表述过重，不应保留。更准确的说法是：

1. `ZlibStreamCompressor` / `ZstdStreamCompressor` 当前会保留本 pending block 已发射的编码输出
2. 峰值内存随单个 pending block 大小增长，而不是随整个 engine 生命周期无限增长
3. 如果后续要继续压低 async 峰值内存，可以评估 writer 直写或显式回收策略

## 3. 测试结论

当前测试覆盖比原始审查里写得更完整：

1. `AppenderEngine` 已有 async flush、startup recover、mode switch、timeout flush 等集成测试
2. `oneshot_flush` 已有端到端恢复测试
3. `mmap_recovery` 已覆盖 torn tail / pending block 恢复

仍值得继续补的主要是：

1. `platform_console.rs` 非 Apple/Android fallback 行为测试
2. 更细粒度的 `FileManager` 分层后测试

## 4. 当前优先级

### P0

1. 维持去重后的 recovery helper 不回退
2. 维持 `FileManager` 单写者锁、文档和回归测试闭环

### P1

1. 拆分 `FileManager`
2. 继续收口类型与错误处理策略

### P2

1. 评估压缩器内存保留策略
2. 增补 `platform_console` fallback 测试
