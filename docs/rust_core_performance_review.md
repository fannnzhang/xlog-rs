# mars-xlog-core 性能与正确性审查

> 审查日期: 2026-03-16
> 范围: `crates/xlog-core/` + `crates/xlog/src/backend/rust.rs`

## 1. 当前结论

当前代码不应再被描述成“Rust 仍系统性落后于 C++”。

当前只保留以下结论：

1. Rust 在现有双端矩阵里已经具备明确性能竞争力
2. sync 吞吐不再是主矛盾
3. async 剩余问题主要集中在 `async_4t_zstd3` tail latency 与小消息 `bytes/msg`
4. Rust 侧 active semantic blocker 已清零，但语义护栏和回归测试必须继续维持

## 2. 权威事实基线

当前性能判断以以下基线为准：

1. 双端矩阵：`artifacts/bench-compare/20260308-p0-full-matrix`
2. Criterion：`artifacts/criterion/20260308-p0-full-review`

当前只保留项目级判断：

1. Rust 整体吞吐和平均延迟已领先
2. `core_formatter` / `core_crypto` 不是主热点
3. async public write path 仍明显重于 sync
4. `core_compress_decode/zstd_*` 高噪声，只能作诊断信号

完整 runner、矩阵与回归治理见 [benchmark_strategy.md](/Users/fannnzhang/.codex/worktrees/6bdf/xlog-rs/docs/benchmark_strategy.md)。

## 3. 正确性状态

当前只保留两条需要持续成立的结论：

1. `FileManager` 单写者约束已经通过 `log_dir/cache_dir` 锁、README 和回归测试显式化
2. recovery / oneshot 的 recovered block 必须保持连续写出，不能退回 split-write

更完整的语义约束见 [rust_semantic_redlines.md](/Users/fannnzhang/.codex/worktrees/6bdf/xlog-rs/docs/rust_semantic_redlines.md)。

## 4. 当前真正值得做的事情

### P0

1. 基于已有 metrics 解释 async `bytes/msg` 与 tail
2. 维持 `FileManager` 单写者护栏、文档和测试闭环

### P1

1. 围绕 `compress.rs` flush 粒度做定向实验
2. 调整 frontend queue drain / backpressure 策略
3. 收敛 `async_4t_zstd3` tail latency

### 当前不建议优先做

1. 继续把 sync 吞吐当成第一性能目标
2. per-thread pending pipeline
3. 大规模 lock-free / SIMD / OS 级 mmap 调优
4. 面向单机型 benchmark 的运行时特调

## 5. 最低回归要求

后续涉及写路径、恢复或文件管理的改动，至少执行：

1. `cargo test -p mars-xlog-core --test async_engine`
2. `cargo test -p mars-xlog-core --test mmap_recovery`
3. `cargo test -p mars-xlog-core --test oneshot_flush`
4. `cargo test -p mars-xlog-core file_manager:: -- --nocapture`
5. `cargo test -p mars-xlog --lib`

如涉及 async 写路径或压缩，还应补跑相应 benchmark / Criterion。
