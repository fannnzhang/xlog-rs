# Xlog Rust 迁移代码 Review 报告（修订版）

> 审查时间：2026-02-28  
> 修订时间：2026-02-28  
> 审查范围：`crates/xlog-core/src/*` + `crates/xlog/src/backend/rust.rs`  
> 对照基线：`third_party/mars/mars/xlog/` C++ 实现

## 1. 结论

当前 Rust 迁移主链路已可运行并通过现有回归，上一版报告中的阻断项已完成收口。

- 阻断级（必须修）：**0 项**
- 中优先级（兼容收敛）：**0 项**
- 保留项：可读性建议、架构差异说明或后续优化项

> 本修订版重点修正了原报告里“问题级别和结论不一致”的内容，只保留可执行结论。

---

## 2. 已完成整改（原 Blocking/P2）

### 2.1 `maintid` 使用 `pid` 的问题已修复

- 文件：`crates/xlog/src/backend/rust.rs`
- 修复：改为缓存主线程 tid（`OnceLock`）并用于 `LogRecord.maintid`
- 结果：跨平台 `tid == maintid` 时可正确输出 `*` 标记

### 2.2 Sync + crypt magic/payload 语义不一致已修复

- 文件：`crates/xlog/src/backend/rust.rs`
- 修复：引入 `is_crypt = cipher.enabled() && mode == Async`
- 结果：sync 模式不再写 crypt magic，且 `client_pubkey` 与 magic 语义一致

### 2.3 formatter body 截断策略已补齐

- 文件：`crates/xlog-core/src/formatter.rs`
- 修复：新增 UTF-8 边界截断（`MAX_LOG_BODY_BYTES = 0xFFFF`）
- 结果：超长消息不再无限放大单条 block

### 2.4 4/5 buffer 告警注入已补齐

- 文件：`crates/xlog-core/src/appender_engine.rs` + `crates/xlog/src/backend/rust.rs`
- 修复：补充 async buffer 统计与高水位告警注入路径
- 结果：高压场景下会产出告警日志并触发强制 flush

### 2.5 Android console 输出已切换 Logcat

- 文件：`crates/xlog-core/src/platform_console.rs`
- 修复：Android 分支改为 `__android_log_write`
- 结果：console 日志路径与平台行为对齐

### 2.6 `oneshot_flush` 截断文件容错已补齐

- 文件：`crates/xlog-core/src/oneshot.rs`
- 修复：`read_to_end` + 不足补零 + 超长截断
- 结果：截断 mmap 文件可恢复，崩溃场景鲁棒性提升

---

## 3. 新增回归验证（已落地）

- `crates/xlog-core/src/formatter.rs`：新增超长 UTF-8 截断单测
- `crates/xlog-core/tests/oneshot_flush.rs`：新增截断 mmap 恢复单测
- `crates/xlog/src/backend/rust.rs`：
  - 新增 sync + pub_key magic/payload 语义单测
  - 新增主线程 `*` 标记行为单测

---

## 4. 非阻断项（保留）

### 4.1 可读性或文档注释项

- `crypto.rs` `tea_decrypt_in_place` 中 `delta << 4` 与 `wrapping_mul(16)` 在 `u32` 上等价
- `encrypt_sync` 不加密与 C++ 当前实现一致（建议仅保留注释说明）

### 4.2 架构差异（已接受）

- 当前 Rust 路径为“每条日志独立 block”，不同于 C++ async 共享压缩流  
  已由 Phase 2C 官方解码回归覆盖，可继续沿用

### 4.3 后续可优化（非本轮阻断）

- `SeqGenerator::next_async` 在通用并发语义上可改 CAS loop；当前调用路径有锁保护
- `registry` 使用 `Weak` 需依赖上层持有强引用（当前 `Xlog` 持有 `Arc`，可工作）
- mmap 每次 append 后 `flush` 性能开销偏高，可后续按批量策略优化

---

## 5. 与迁移计划的对应关系（更新后）

本报告对应 `docs/xlog_rust_migration_plan.md` 的收敛项应纳入：

1. **Phase 4 收口**：本报告原 P0/P2 项已关闭
2. **Phase 5 灰度前门槛**：review 阻断项清零，保持回归脚本持续运行
3. **Phase 6 前置条件**：保持 review 阻断项为 0 并继续做性能/构建收敛

---

## 6. 建议继续跟进

- Android 真机验证 Logcat 输出链路（本次代码已切换，建议补 CI/设备验证）
- 高压场景进一步验证 4/5 告警触发频次与可观测性
- 持续跟踪 Rust/FFI 性能差距并推进优化
