# Rust 的探针技术

## 核心问题

Rust 有探针技术吗？答案是**有**，但与 Java 的 Agent 技术有本质区别。

**关键区别**：Java 是运行在 JVM 上的语言，JVM 提供了字节码注入的 API（Java Agent），可以在运行时修改类加载的字节码。Rust 是编译型语言，直接编译成机器码，没有虚拟机层提供这样的机制。

---

## Rust 探针技术体系

| 技术 | 类型 | 用途 |
|------|------|------|
| `tracing` crate | 编译时插桩 | 结构化日志、span 追踪 |
| `tokio-console` | 运行时调试 | async runtime 状态分析 |
| `miri` | 解释器检测 | 内存错误、UB 检测 |
| `cargo-flamegraph` | 性能分析 | CPU 火焰图生成 |
| `perf` / `bpftrace` | 系统级探针 | Linux 内核级追踪 |
| `#[instrument]` | 编译时插桩 | 函数入口/出口自动追踪 |

---

## tracing crate：结构化日志与 Span 探针

### 什么是 tracing？

`tracing` 是 Rust 的结构化日志库，类似于 Go 的 `log/slog` 或 Java 的 `Log4j`，但更强大——它支持 **span** 概念，可以追踪请求在整个系统中的传播路径。

### 安装

```toml
[dependencies]
tracing = "0.1"
tracing-subscriber = "0.3"
```

### 基本用法：函数入口追踪

```rust
use tracing::{instrument, info, warn};

#[instrument]
fn process_order(order_id: u64) -> Result<(), String> {
    info!("Processing order started");

    let result = fetch_order(order_id)?;

    if result.is_overdue() {
        warn!("Order is overdue");
    }

    info!("Processing order completed");
    Ok(())
}
```

**`#[instrument]` 的作用**：编译器自动在函数入口和出口插入 span，记录函数名、参数、返回值、执行时长。

### Span：请求链路追踪

```rust
use tracing::{info, span, Level};

fn handle_request(request: Request) {
    // 创建 span
    let span = span!(Level::INFO, "handle_request", request_id = %request.id);

    // 进入 span
    let _guard = span.enter();

    info!("Request received");

    process_database(&request);   // 子 span 自动创建
    process_external_api(&request); // 子 span 自动创建

    info!("Request completed");
    // _guard drop，span 结束
}
```

### Span 的层级结构

```
[handle_request]  ← 根 span
  ├── [process_database]  ← 子 span
  │     └── [db_query]
  └── [process_external_api]  ← 子 span
```

### tracing-subscriber：导出追踪数据

```rust
use tracing_subscriber::{fmt, prelude::*};

fn main() {
    // 格式化输出到 stdout
    tracing_subscriber::registry()
        .with(fmt::layer())
        .init();

    // 追踪数据会被输出
    info!("This is a trace event");
}
```

### 导出到 Jaeger（分布式追踪）

```toml
[dependencies]
tracing-jaeger = "0.2"
```

```rust
use tracing_jaeger::new_pipeline;

fn main() {
    let (tracer, task) = new_pipeline()
        .with_service_name("my-service")
        .install()
        .expect("Failed to install tracer");

    // 或者异步处理
    // tokio::spawn(task);

    // 追踪数据会发送到 Jaeger
    tracing::info!("Tracing to Jaeger");
}
```

---

## tokio-console：Async Runtime 调试

### 什么是 tokio-console？

`tokio-console` 是用于调试 Tokio async runtime 的工具，可以查看：
- 当前活跃的 task
- task 的状态（running, idle, blocked）
- 资源等待情况（锁、channel 等）

### 安装

```bash
cargo install tokio-console
```

### 使用

```rust
// 在代码中添加 console feature
// Cargo.toml:
// tokio = { version = "1", features = ["full", "console"] }
```

```bash
# 运行你的程序
cargo run

# 在另一个终端启动 console
tokio-console
```

### 能看到什么？

```
tokio-console 截图示例：
├── my-service
│   ├── Task 1 (running) [task_id=1]
│   ├── Task 2 (blocked) [task_id=2] - waiting on Mutex
│   └── Task 3 (idle) [task_id=3]
```

---

## cargo-flamegraph：CPU 火焰图

### 安装

```bash
cargo install cargo-flamegraph
```

### 使用

```bash
# 生成火焰图
cargo flamegraph --bin my-binary

# 生成特定函数的火焰图
cargo flamegraph -c "my_function" --bin my-binary
```

### 输出

生成 `flamegraph.svg`，可以用浏览器打开查看调用栈。

---

## Miri：内存错误检测

### 安装

```bash
rustup component add miri
```

### 运行

```bash
cargo miri run
```

### 检测的问题

| 问题类型 | 说明 |
|---------|------|
| 使用未初始化内存 | `let x: i32; println!("{}", x);` |
| 越界访问 | `arr[100]` 超出范围 |
| 悬空指针 | `&x` 后 x 被释放 |
| 双重释放 | 同一内存 free 两次 |

---

## 系统级探针：perf 与 bpftrace

### perf

```bash
# 编译时加上 debug info
RUSTFLAGS="-C debuginfo=2" cargo build

# 用 perf 采样
sudo perf record -g --call-graph dwarf ./target/debug/my-binary

# 生成火焰图
sudo perf script | inferno-collapse-perf | inferno-flamegraph > flamegraph.svg
```

### bpftrace（Linux）

```bash
# 追踪 Rust 程序中的函数调用
sudo bpftrace -e '
  uretprobe:/path/to/my-binary:function_name {
    printf("Function returned: %d\n", retval);
  }
'
```

---

## 与 Java Agent 的对比

| 维度 | Java Agent | Rust 探针 |
|------|-----------|-----------|
| 机制 | 字节码注入 | 编译时插桩 / 运行时追踪 |
| 运行时修改 | 支持 | 不支持 |
| 字节码编织 | 支持 | 不支持（Rust 是编译型） |
| 典型用途 | AOP, APM 探针 | 日志、追踪、性能分析 |
| 框架 | ByteBuddy, ASM | tracing, tokio-console |

### 为什么 Rust 没有 Java Agent 那样的技术？

1. **编译型 vs 解释型**：Java 编译成字节码在 JVM 上运行，JVM 提供类加载时修改字节码的 API。Rust 直接编译成机器码，没有中间层。

2. **所有权系统**：Rust 的借用检查器在编译时确保内存安全，运行时注入字节码会破坏这种保证。

3. **替代方案**：Rust 的方案是**编译时插桩**（`#[instrument]`）和**结构化日志**（`tracing`），在编译期就确定追踪逻辑，零运行时开销。

---

## 最佳实践

### 1. 使用 tracing 进行结构化日志

```rust
use tracing::{info, instrument};

#[instrument(skip(data), fields(data_len = data.len()))]
fn process(data: &[u8]) -> Result<(), Error> {
    info!("Processing data");
    // ...
}
```

### 2. 使用 tokio-console 调试 async

```bash
# 开发时运行
RUSTFLAGS="--cfg tokio_unstable" cargo run

# 查看 async task 状态
tokio-console
```

### 3. 使用 cargo-flamegraph 分析性能

```bash
cargo flamegraph --bin my-service
```

---

## 总结

| 需求 | Rust 方案 |
|------|----------|
| 结构化日志 | `tracing` crate |
| 分布式追踪 | `tracing` + Jaeger/Zipkin |
| Async 调试 | `tokio-console` |
| 内存错误检测 | `miri` |
| CPU 性能分析 | `cargo-flamegraph`, `perf` |
| 系统级探针 | `perf`, `bpftrace` |

**核心结论**：Rust 没有 Java Agent 那样的字节码注入技术，但有更轻量的**编译时插桩**方案，零运行时开销。
