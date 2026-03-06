# mars-xlog-core 深度性能审查报告

> 审查日期: 2026-03-06
> 审查范围: `crates/xlog-core/` 全部 16 个源文件 + `crates/xlog/src/backend/rust.rs` 集成层
> 审查视角: Rust 语言特性 × 计算机工程性能优化

---

## 一、关键热路径分析

每次 `write` 调用的完整路径：

```
Xlog::write
  → RustBackend::write_with_meta_internal
    → level check (AtomicI32 Relaxed load)
    → resolve_record_meta (pid/tid/maintid)
    → [Sync]  format → build_sync_block → engine.write_block → file_manager.append
    → [Async] format → checkout_async_state(LOCK) → compress → encrypt → engine.append(LOCK) → maybe flush
```

### Async 热路径锁序列

```
调用线程:
  1. thread_local HOT_PATH_SCRATCH (RefCell borrow)      — 无竞争
  2. format_record_parts_into()                            — 并行
  3. async_state Mutex + Condvar (checkout)                 — 独占，串行点 ①
     ├─ compress_chunk (流式压缩)
     ├─ encrypt_async_in_place (TEA 加密)
     └─ engine.state Mutex (buffer append)                 — 串行点 ②
  4. release async_state → notify_one

后台 worker 线程:
  5. engine.state Mutex (flush pending → file I/O)         — 串行点 ②
```

### Sync 热路径锁序列

```
调用线程:
  1. thread_local HOT_PATH_SCRATCH                         — 无竞争
  2. format + build_sync_block                             — 并行
  3. engine.state Mutex (clone FileManager + max_file_size) — 短暂持有
  4. FileManager.runtime Mutex (file I/O)                  — 串行点，持有期间执行文件写入
```

---

## 二、发现的性能问题

### P0：关键瓶颈

#### P0-1: Zlib 压缩级别硬编码为最高级别

**文件:** `crates/xlog-core/src/compress.rs:29-35`

```rust
impl Default for ZlibStreamCompressor {
    fn default() -> Self {
        Self {
            inner: flate2::write::DeflateEncoder::new(Vec::new(), Compression::best()),
            emitted: 0,
        }
    }
}
```

**问题:** `Compression::best()` = zlib level 9，是最慢的压缩级别。`XlogConfig.compress_level` 默认值为 6，但 `ZlibStreamCompressor::default()` 完全忽略了这个配置。zlib level 6 通常比 level 9 快 2-3 倍，压缩率仅损失 1-3%。

**调用链:** `RustBackend::new_async_pending_state` → `ZlibStreamCompressor::default()` — 永远使用 level 9。

**优化方案:**
- `ZlibStreamCompressor` 增加 `new(level: u32)` 构造方法
- 在 `RustBackend::new_async_pending_state` 中传递 `config.compress_level`

**预估收益:** Async 单线程写入速度提升 2-3 倍（压缩是 async 热路径中 CPU 占比最高的操作）。

---

#### P0-2: 流式压缩器双缓冲 (Double Buffering)

**文件:** `crates/xlog-core/src/compress.rs:38-64`, `crates/xlog-core/src/compress.rs:116-134`

```rust
pub struct ZlibStreamCompressor {
    inner: flate2::write::DeflateEncoder<Vec<u8>>,  // 内部缓冲 ①
    emitted: usize,
}

fn compress_chunk(&mut self, input: &[u8], output: &mut Vec<u8>) -> Result<(), CompressError> {
    self.inner.write_all(input)?;
    self.inner.flush()?;
    let encoded = self.inner.get_ref();
    if encoded.len() > self.emitted {
        output.extend_from_slice(&encoded[self.emitted..]);  // 拷贝到外部缓冲 ②
        self.emitted = encoded.len();
    }
    Ok(())
}
```

**问题:** 压缩后的数据经历两次缓冲：

1. `DeflateEncoder` 写入其内部 `Vec<u8>` → 第一次缓冲
2. 从 `self.emitted` 偏移拷贝到 `output` → 第二次缓冲（memcpy）

内部 `Vec<u8>` 在 pending block 生命周期内持续增长，不被释放。对于 150KB 的 buffer，压缩后数据可能占 50-100KB，全部累积在内部 Vec 中直到 block finalize。

`ZstdStreamCompressor` 存在相同问题。

**优化方案:**
- 实现自定义 `Write` 适配器，让 `DeflateEncoder` 直接追加写入 `output` Vec
- 通过 wrapper 追踪 emitted 位置，避免内部累积缓冲区

**预估收益:** 减少 50-100KB memcpy / pending block + 降低内存峰值。

---

#### P0-3: mmap flush 使用同步 msync

**文件:** `crates/xlog-core/src/mmap_store.rs:88-93`

```rust
pub fn flush(&mut self) -> Result<(), MmapStoreError> {
    self.mmap
        .flush()
        .map_err(|e| MmapStoreError::Flush(self.path.clone(), e))
}
```

**问题:** `MmapMut::flush()` 调用 `msync(MS_SYNC)`，是阻塞系统调用，强制脏页写入存储介质后才返回。在 async 的周期性持久化中（`should_persist_async_mmap` 触发，非 force_flush），使用同步 flush 会不必要地阻塞调用线程。

当前的持久化节流策略已经很合理：

```rust
const ASYNC_PENDING_MMAP_PERSIST_EVERY_UPDATES: u32 = 32;
const ASYNC_PENDING_MMAP_PERSIST_EVERY_BYTES: usize = 32 * 1024;
const ASYNC_PENDING_MMAP_PERSIST_INTERVAL: Duration = Duration::from_millis(250);
```

但每次触发时的 `msync(MS_SYNC)` 在移动设备 flash 存储上可能耗时 1-10ms。

**优化方案:**
- `MmapStore` 增加 `flush_async()` 方法，调用 `self.mmap.flush_async()` (`msync(MS_ASYNC)`)
- 在非强制持久化（`should_persist_async_mmap` 且 `!force_flush`）时使用 `flush_async`
- 仅在 `force_flush=true`、shutdown、mode switch 时使用同步 `flush`

**预估收益:** 减少 async 写入路径中的 I/O 等待时间，尤其在 flash 存储设备上。

---

#### P0-4: Async 状态锁持有范围过大

**文件:** `crates/xlog/src/backend/rust.rs:546-626`

```rust
fn write_async_line(&self, ...) {
    with_hot_path_scratch(|scratch| {
        self.format_record_line_into(&mut scratch.line, ...);  // 并行部分

        let mut checked_out = self.checkout_async_state();  // ← 获取独占锁

        // 以下全部在锁内执行:
        // - 流式压缩 (CPU 密集)
        // - TEA 加密 (CPU 密集)
        // - engine.append_async_chunk (获取 engine state 锁)
        state.append_chunk(...);

        // ← 锁在 checked_out drop 时释放
    });
}
```

`checkout_async_state` 使用 `Mutex + Condvar` 独占流式压缩器，锁的持有范围覆盖了 **压缩 + 加密 + engine buffer 追加**。这意味着多线程 async 写入在压缩/加密阶段完全串行化，无法利用多核并行。

**优化方案（短期）:**
- 将锁的粒度细化：先在锁外做格式化，进入锁后只做压缩+加密+append
- 当前实现已经在锁外做格式化，瓶颈在于压缩占锁时间长

**优化方案（中期 - 架构调整）:**
- 采用 per-thread pending block 方案：每线程维护独立的 compressor + pending state
- flush 时将所有线程的 pending blocks 按序合并写入 mmap buffer
- 这需要修改 async 协议模型，但可以完全消除压缩阶段的锁竞争

**预估收益:** Async 4T 场景吞吐量可能提升 30-60%（取决于压缩占总延迟的比例）。

---

### P1：重要优化

#### P1-1: `current_tid()` 重复系统调用

**文件:** `crates/xlog-core/src/platform_tid.rs:3-22`

```rust
pub fn current_tid() -> i64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        unsafe { libc::syscall(libc::SYS_gettid) as i64 }
    }
    #[cfg(any(target_os = "macos", ...))]
    {
        let mut tid: u64 = 0;
        unsafe { libc::pthread_threadid_np(0, &mut tid); }
        tid as i64
    }
}
```

**问题:** 每条日志调用一次 `syscall(SYS_gettid)` 或 `pthread_threadid_np`。线程 ID 在线程生命周期内不变，无需反复查询。

**优化方案:**

```rust
pub fn current_tid() -> i64 {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        thread_local! {
            static TID: i64 = unsafe { libc::syscall(libc::SYS_gettid) as i64 };
        }
        TID.with(|t| *t)
    }
    // ...
}
```

**预估收益:** 每条日志减少 1 次系统调用。

---

#### P1-2: 时间格式化效率低

**文件:** `crates/xlog-core/src/formatter.rs:33-48`

```rust
fn format_time_into(out: &mut String, ts: std::time::SystemTime) {
    let dt: DateTime<Local> = ts.into();  // timezone 查找
    let offset_hours = (dt.offset().local_minus_utc() as f64) / 3600.0;  // 浮点除法
    let _ = write!(out, "{:04}-{:02}-{:02} {:+.1} {:02}:{:02}:{:02}.{:03}", ...);
}
```

**问题:**
1. `DateTime<Local>` 转换涉及 timezone 查找（某些平台读 `/etc/localtime`）
2. `offset_hours` 使用 f64 除法（不必要的精度）
3. `write!` 宏每个数字都经过 `Display` trait，效率低于直接字符串操作

每条日志都执行一次这些操作。

**优化方案:**
- 使用 `thread_local!` 缓存 UTC offset（每小时或每分钟刷新）
- 使用 `itoa` crate 或手动定宽数字格式化替代 `write!` 宏
- 预计算 `offset_hours` 避免每条日志的浮点除法

**预估收益:** 格式化阶段耗时减少 30-50%。

---

#### P1-3: mmap 预分配使用堆分配 Vec

**文件:** `crates/xlog-core/src/mmap_store.rs:104-111`

```rust
fn preallocate_by_zero_write(file: &mut File, capacity: usize) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    let zeros = vec![0u8; capacity];  // 150KB 堆分配
    file.write_all(&zeros)?;
    file.flush()?;
    Ok(())
}
```

**问题:** 每次创建 mmap 文件时在堆上分配 150KB 的零值 Vec，只用一次就释放。

**优化方案:**

```rust
fn preallocate_by_zero_write(file: &mut File, capacity: usize) -> std::io::Result<()> {
    file.seek(SeekFrom::Start(0))?;
    let zeros = [0u8; 8192];
    let mut remaining = capacity;
    while remaining > 0 {
        let n = remaining.min(zeros.len());
        file.write_all(&zeros[..n])?;
        remaining -= n;
    }
    file.flush()?;
    Ok(())
}
```

进阶方案: Linux/Android 上使用 `fallocate(FALLOC_FL_ZERO_RANGE)`。

**预估收益:** 初始化阶段减少 150KB 堆分配（一次性，非热路径但仍有意义）。

---

#### P1-4: 文件扩展名字符串重复分配

**文件:** `crates/xlog-core/src/file_manager.rs` 多处

```rust
// 出现在 list_existing_files, move_old_cache_files, get_file_names_by_prefix 中
if name.starts_with(file_prefix) && name.ends_with(&format!(".{LOG_EXT}"))
```

**问题:** 每次目录遍历的每个条目都 `format!` 分配一个新 String `".xlog"`。

**优化方案:**

```rust
const LOG_EXT_DOT: &str = ".xlog";
// 替换所有 format!(".{LOG_EXT}") 为 LOG_EXT_DOT
```

**预估收益:** 消除目录遍历中每条目的堆分配。

---

#### P1-5: `Local::now()` 和 `SystemTime::now()` 冗余双重调用

**文件:** `crates/xlog/src/backend/rust.rs:558-560`

```rust
let now = Local::now();              // 调用 ① clock_gettime + tz lookup
let now_hour = chrono::Timelike::hour(&now) as u8;
let timestamp = std::time::SystemTime::now();  // 调用 ② clock_gettime
```

**问题:** 两次独立的时钟调用获取本质上相同的时间点。

**优化方案:**

```rust
let timestamp = std::time::SystemTime::now();
let dt: DateTime<Local> = timestamp.into();
let now_hour = dt.hour() as u8;
```

`build_sync_block_into` (第 369-381 行) 中也存在相同问题。

**预估收益:** 每条日志减少 1 次 `clock_gettime` 系统调用。

---

### P2：架构级优化

#### P2-1: Sync 模式多线程 FileManager 锁竞争

**文件:** `crates/xlog-core/src/file_manager.rs`

**问题:** `FileManager.runtime` 是 `Arc<Mutex<RuntimeState>>`，sync 模式的 `append_log_bytes` 在持有该锁期间执行文件 I/O。多线程 sync 写入完全串行化在 FileManager 的 Mutex 上。

从 benchmark 数据看：Sync 4T Rust ≈ 81.8% of C++，这个差距的主要原因就是锁竞争。

**优化方案（高复杂度）:**
- 使用 OS 级 `O_APPEND` 原子追加语义，每线程持有独立文件句柄
- 通过 `pwrite` + `O_APPEND` 保证并发写入的原子性和顺序
- 路径选择和轮转逻辑使用 `RwLock` 或 lock-free 设计

**风险:** 需要处理文件轮转（rotation）时的同步问题。

---

#### P2-2: Async 模式 per-thread 压缩管道

**文件:** `crates/xlog/src/backend/rust.rs`

**问题:** 当前所有线程共享一个 `AsyncPendingState`（包含流式压缩器），通过 Mutex + Condvar 串行化。这限制了 async 多线程的并行度。

**优化方案（高复杂度）:**
- 每线程维护独立的 `AsyncPendingState`（独立的压缩器实例）
- 每线程的 compressed+encrypted 输出写入独立的小缓冲区
- flush 时按序合并所有线程的输出到 mmap buffer
- 需要修改协议模型：从「单 pending block」变为「多 pending block」

**风险:** 修改了 async block 语义，需要验证解码兼容性。

---

#### P2-3: scan_recovery 尾部扫描 O(n)

**文件:** `crates/xlog-core/src/buffer.rs:394`

```rust
let dropped_nonzero_tail_bytes = raw[offset..].iter().filter(|b| **b != 0).count();
```

**问题:** 对于 150KB 的 buffer，如果 valid_len 只有几 KB，剩余约 147KB 全部被扫描。这只是诊断信息，不影响正确性。

**优化方案:**
- 惰性求值：只在实际访问 `dropped_nonzero_tail_bytes` 时才计算
- 或设置扫描上限：`raw[offset..].iter().take(4096).filter(...)` 采样前 4KB 即可判断是否有脏数据

---

#### P2-4: `append_file_to_file` 缓冲区过小

**文件:** `crates/xlog-core/src/file_manager.rs:1034`

```rust
let mut buf = [0u8; 4096];
```

**问题:** 4KB 的读写缓冲区意味着每 4KB 就需要一次 `read` + `write` 系统调用对。对于较大的日志文件合并，系统调用次数可能很高。

**优化方案:** 增大到 64KB：

```rust
let mut buf = [0u8; 65536];
```

或在 Linux 上使用 `copy_file_range` / `sendfile` 系统调用实现零拷贝。

---

### P3：微优化

#### P3-1: TEA 加密可考虑 SIMD 或 unsafe 优化

**文件:** `crates/xlog-core/src/crypto.rs:143-167`

当前实现使用 `chunks_exact_mut(8)` + 手动字节到 u32 转换，safe Rust 编译器一般能很好地优化。但在 ARM NEON（Android 主要平台）上，可以考虑 SIMD 批量加密提高吞吐。

属于极限优化，优先级低。

#### P3-2: `LogRecord` 拥有 String 字段

**文件:** `crates/xlog-core/src/record.rs:33-42`

```rust
pub struct LogRecord {
    pub tag: String,
    pub filename: String,
    pub func_name: String,
    // ...
}
```

热路径实际使用 `format_record_parts_into` 直接传入 `&str`，绕过了 `LogRecord` 构造。当前不是性能问题，但如果未来有代码路径需要构造 `LogRecord`，应考虑使用 `Cow<'a, str>` 避免不必要的拷贝。

---

## 三、优化优先级矩阵

| 优先级 | ID | 优化项 | 预估收益 | 实现复杂度 | 兼容性风险 |
|--------|----|--------|---------|-----------|-----------|
| **P0** | P0-1 | Zlib 压缩级别传入配置 | Async 写入 2-3x | 低 | 无 |
| **P0** | P0-2 | 流式压缩器消除双缓冲 | 减少 memcpy + 内存 | 中 | 无 |
| **P0** | P0-3 | mmap flush_async 替代 flush | 减少 I/O 阻塞 | 低 | 低 |
| **P0** | P0-4 | 缩减 async 状态锁持有范围 | Async 4T 提升 30-60% | 高 | 中 |
| **P1** | P1-1 | Thread-local tid 缓存 | 减少 syscall/line | 低 | 无 |
| **P1** | P1-2 | 时间格式化优化 | Format 阶段 30-50% | 中 | 无 |
| **P1** | P1-3 | mmap 预分配用栈缓冲区 | 减少堆分配 | 低 | 无 |
| **P1** | P1-4 | 文件扩展名常量化 | 消除热路径分配 | 低 | 无 |
| **P1** | P1-5 | 合并冗余时钟调用 | 减少 syscall/line | 低 | 无 |
| **P2** | P2-1 | Sync per-thread 文件句柄 | Sync 4T 吞吐量大幅提升 | 高 | 高 |
| **P2** | P2-2 | Async per-thread 压缩管道 | Async 4T 吞吐量大幅提升 | 高 | 高 |
| **P2** | P2-3 | scan_recovery 惰性尾部计算 | 减少不必要计算 | 低 | 无 |
| **P2** | P2-4 | append_file_to_file 增大 buffer | 减少 syscall 次数 | 低 | 无 |

---

## 四、Benchmark 现状与优化目标

### 当前性能 (基于 20260306-harness-matrix-rerun 数据)

| 场景 | Rust / C++ 比值 | 瓶颈分析 |
|------|----------------|---------|
| Async 1T | ≈ 81% | 压缩级别 (P0-1) + 双缓冲 (P0-2) |
| Async 4T | ≈ 110.7% | 已优于 C++，P0-4 可进一步提升 |
| Async 4T + flush/256 | ≈ 97.3% | flush 路径 msync 开销 (P0-3) |
| Sync 1T | ≈ 111.7% | 已优于 C++ |
| Sync 4T | ≈ 81.8% | FileManager 锁竞争 (P2-1) |
| Sync 4T + cache | 优于 C++ | 特定场景已优 |
| Sync 4T + boundary | 远优于 C++ | C++ 在此场景有已知问题 |

### P0 优化完成后预期目标

| 场景 | 预期 Rust / C++ 比值 |
|------|---------------------|
| Async 1T | **100-130%** (主要来自 P0-1 压缩级别修正) |
| Async 4T | **120-150%** (P0-4 锁优化) |
| Sync 4T | 81-85% (需 P2 级别重构才能显著提升) |

---

## 五、实施建议

### 第一阶段：低风险高收益 (P0-1, P0-3, P1-1, P1-3, P1-4, P1-5)

这些修改实现简单、无兼容性风险，可以快速验证收益。建议先实施后跑 benchmark 确认提升幅度。

### 第二阶段：中等复杂度 (P0-2, P1-2)

流式压缩器重构和时间格式化优化需要更仔细的测试，但风险可控。

### 第三阶段：架构调整 (P0-4, P2-1, P2-2)

这些需要修改核心并发模型，建议先做详细设计，评估对协议兼容性和错误恢复的影响。
