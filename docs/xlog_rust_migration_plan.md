# Xlog Rust 当前状态与下一步计划

> 更新日期: 2026-03-16

## 1. 当前状态

截至当前代码：

1. Rust backend 已是主实现路径
2. Rust 侧 active semantic blocker 已清零
3. 当前仍不能进入移除 C++ 依赖阶段

## 2. 下一步计划

### P0

1. 维持 `FileManager` 单写者约束闭环
2. 持续保持 recovery / oneshot contiguous append 语义
3. 基于已有 metrics 解释 async 剩余差距

### P1

1. 定向治理 `async_4t_zstd3` tail latency
2. 定向治理 async 小消息 `bytes/msg`
3. 拆分 `FileManager` 降低维护成本

## 3. 约束

后续优化不得跨越以下边界：

1. 不改协议兼容性
2. 不弱化恢复语义
3. 不把 sync / fatal 写成更强语义
4. 不移除 single-writer guardrails
5. 不引入面向单机型 benchmark 的 runtime 特调

详细约束见 [rust_semantic_redlines.md](/Users/fannnzhang/.codex/worktrees/6bdf/xlog-rs/docs/rust_semantic_redlines.md)。
