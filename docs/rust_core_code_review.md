# mars-xlog-core 代码架构与可维护性审查

> 审查日期: 2026-03-16
> 复核日期: 2026-03-17
> 审查范围: `crates/xlog-core/src/`

## 当前结论

原始 review 中除 `FileManager` 复杂度外的其余问题，当前都已经处理或证伪，不再保留为 active issue。

已经关闭或收口的内容包括：

1. recovery helper 重复实现
2. `oneshot_flush` 缺少端到端测试
3. `AppenderEngine` 缺少 async/sync 集成测试
4. mutex poison 在关键 setter / 查询路径上的不一致
5. header 偏移 magic number
6. `oneshot.rs` 的 `u64 -> usize` 截断问题
7. 热路径上重复 `format!(".{LOG_EXT}")`
8. `mmap_store` 的整块零填充分配
9. `ZlibStreamCompressor` 缺少 roundtrip 测试
10. 压缩器保留整块已发射输出
11. `ConsoleLevel` / `LogLevel` 重复类型
12. `FileManager` 命名层、writer 层、plain/cache 主路径未拆分

## 唯一剩余问题

`FileManager` 仍然把 cache/log 生命周期策略与业务路由协调耦合在同一个模块里。

更具体地说：

1. 命名层已经拆到 `file_naming.rs`
2. buffered writer 已拆到 `active_append.rs`
3. plain/cache 主路径已经拆开
4. `RuntimeState` / `AppendTargetCache` 已抽到独立的 `file_runtime.rs`
5. 但 cache 命中、cache 提升、log fallback 和文件生命周期策略仍由 `file_manager.rs` 统一决策

因此当前 `mars-xlog-core` 在可维护性上的唯一主要剩余问题是：

`FileManager` 的问题已经不再是命名、writer 或状态机细节，而是 cache/log 生命周期策略与路由决策仍过于耦合。

## 后续方向

下一步如果继续重构，应只围绕这一个问题展开：

1. 继续收口 `AppendTargetCache` 相关辅助逻辑
2. 把 cache 命中 / promote / fallback 进一步收成显式策略边界
3. 在拆分后补更细粒度的 `FileManager` 分层测试
