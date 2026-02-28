# Xlog Rust 迁移代码 Review 报告（修订版）

> 审查时间：2026-02-28  
> 修订时间：2026-02-28  
> 审查范围：`crates/xlog-core/src/*` + `crates/xlog/src/backend/rust.rs`  
> 对照基线：`third_party/mars/mars/xlog/` C++ 实现

## 1. 结论

当前 Rust 迁移主链路已可运行并通过现有回归，但仍有少量兼容性收敛项需要补齐。

- 阻断级（必须修）：**2 项 P0**
- 中优先级（建议本轮修）：**4 项 P2**
- 其余条目为可读性建议、架构差异说明或已接受偏差

> 本修订版重点修正了原报告里“问题级别和结论不一致”的内容，只保留可执行结论。

---

## 2. 必须修复（Blocking）

### P0-1 `maintid` 使用了 `pid`（会导致主线程 `*` 标记异常）

- 文件：`crates/xlog/src/backend/rust.rs`
- 现状：`LogRecord { maintid: pid }`
- 影响：在 macOS/iOS 等平台 `pid != tid`，`[pid, tid*]` 的 `*` 可能长期缺失
- 修复建议：缓存进程启动时主线程 tid，写日志时使用该值填充 `maintid`

### P0-2 Sync + crypt 场景 magic 与 payload 语义不一致

- 文件：`crates/xlog/src/backend/rust.rs`
- 现状：
  - sync 模式下 `encrypt_sync` 返回明文（与 C++ 当前行为一致）
  - 但 header magic 的 crypt 位仍按 `cipher.enabled()` 计算
- 影响：解码器可能按“加密块”路径处理明文 payload，造成兼容风险
- 修复建议：sync 模式下强制 `is_crypt = false`，或显式复刻 C++ 当前“sync 不加密且非 crypt magic”语义

---

## 3. 中优先级收敛项（建议本轮修）

### P2-1 `formatter` 缺少 body 截断策略

- 文件：`crates/xlog-core/src/formatter.rs`
- 现状：body 无长度保护，超长消息可直接拼接
- 风险：极端大消息会放大单条 block，偏离 C++ 的保守截断行为
- 建议：补齐与 C++ 相近的上限策略（至少限制 body 上限并保留尾部换行）

### P2-2 `appender_engine` 未实现 4/5 buffer 告警注入

- 文件：`crates/xlog-core/src/appender_engine.rs`
- 现状：实现了 1/3 唤醒阈值，但未注入 C++ 的 4/5 fatal 提示日志
- 风险：高压下缺少观测信号，不利于问题定位

### P2-3 Android console 输出仍走 `println!`

- 文件：`crates/xlog-core/src/platform_console.rs`
- 现状：Android 分支为 `println!`
- 风险：日志不一定进入 Logcat
- 建议：切换到 `__android_log_write`（或等效封装）

### P2-4 `oneshot_flush` 对截断 mmap 文件容错不足

- 文件：`crates/xlog-core/src/oneshot.rs`
- 现状：`read_exact` 读取固定容量，文件变短时直接失败
- 风险：异常中断后恢复成功率下降
- 建议：改为 `read_to_end` + 不足部分补零

---

## 4. 非阻断项（已复核）

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

## 5. 与迁移计划的对应关系

本报告对应 `docs/xlog_rust_migration_plan.md` 的收敛项应纳入：

1. **Phase 4 收口**：P0-1、P0-2、P2-1、P2-2、P2-3、P2-4
2. **Phase 5 灰度前门槛**：以上 6 项需关闭，且补齐回归测试
3. **Phase 6 前置条件**：P0 必须为 0，P2 至少有明确验收结论

---

## 6. 建议新增回归测试

- sync + pub_key 场景：验证 magic / 解码链路
- 主线程标记场景：验证 `[pid, tid*]` 与跨平台行为
- 超长 body 场景：验证截断策略与协议合法性
- 截断 mmap 恢复场景：验证 `oneshot_flush` 容错
- Android console 场景：验证 Logcat 可见性

