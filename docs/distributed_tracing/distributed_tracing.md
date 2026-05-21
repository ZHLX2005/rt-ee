# Rust 链路追踪生态

## 设计背景与问题域

在分布式系统中，一个用户请求可能跨越数十个微服务。当系统出现故障或性能问题时，如何快速定位问题发生在哪个服务、哪个函数、哪行代码？

这就是**分布式链路追踪（Distributed Tracing）**要解决的问题。

Rust 的链路追踪生态有其独特的设计哲学，这不是简单的"移植 OpenTelemetry 到 Rust"，而是围绕 Rust 的核心特性（所有权、RAII、零成本抽象）重新设计的诊断系统。

要理解这个生态，需要回答：

1. **为什么 Rust 需要 `tracing` 而不是直接用 `log`？**
2. **`tracing` 的 Span 模型与 Java OpenTelemetry 有什么本质区别？**
3. **Rust 如何在异步场景下安全地传播追踪上下文？**
4. **`tracing-subscriber` 的 Layer 架构为什么是"组合优于继承"的典范？**
5. **Rust 的 tracing 生态如何与 Jaeger/Zipkin/Prometheus 等后端集成？**

---

## 核心问题：为什么 tracing 不是"更好的日志库"

### log crate 的局限

传统的日志库（如 Rust 的 `log`、`env_logger`，Java 的 SLF4J/Log4j，Go 的标准库 `log`）有一个共同假设：**日志是平面的**。

```rust
// log crate：每个事件孤立
info!("请求开始, request_id=123");
info!("查询数据库, request_id=123");
info!("请求完成, request_id=123");
```

问题：
- **重复数据**：`request_id` 在每个日志行重复
- **无结构关系**：三行日志在日志系统中是平等的，没有"包含"关系
- **难以聚合**：要从数百万行日志中提取一个请求的完整链路，成本极高

### tracing 的洞察：日志应该是有结构的

`tracing` 的核心设计洞察来自 Google 的 Dapper 论文和 OpenTelemetry 规范：**追踪数据天然是树状的**。

```
[handle_request]              ← Root Span（一段时间范围）
  ├── [authenticate]          ← Child Span
  │     └── [db_query]        ← Grandchild Span
  └── [process_order]         ← Child Span
        └── [http_call]       ← Grandchild Span
```

**Span**：代表一段时间内的操作，有开始、结束和持续时间。
**Event**：代表一个时间点上发生的事情，必须发生在某个 Span 的上下文中。

```rust
use tracing::{info, info_span};

let span = info_span!("handle_request", request_id = 123);
let _guard = span.enter();

info!("请求开始");  // Event，自动关联到 handle_request span
{
    let db_span = info_span!("db_query");
    let _db_guard = db_span.enter();
    info!("查询数据库");  // Event，关联到 db_query span
}
info!("请求完成");
```

输出不再是平面的文本行，而是结构化的数据：
```json
{
  "span": "handle_request",
  "request_id": 123,
  "events": [
    { "msg": "请求开始" },
    {
      "span": "db_query",
      "events": [
        { "msg": "查询数据库" }
      ]
    },
    { "msg": "请求完成" }
  ]
}
```

---

## 核心抽象设计分析

### Span 的 RAII 设计：所有权的妙用

```rust
let span = info_span!("process_order");
let _guard = span.enter();  // span 进入"活跃"状态
// ... 操作 ...
// _guard 在这里 drop，span 自动退出
```

**为什么这是 Rust 独有的优势？**

| 语言 | Span 生命周期管理 | 问题 |
|------|-----------------|------|
| Java | `try (Scope scope = span.makeCurrent()) { ... }` | 依赖 try-with-resources，忘记关闭会导致上下文泄漏 |
| Go | `defer span.End()` | 依赖 defer，但 panic 时可能不执行 |
| **Rust** | **`let _guard = span.enter();`** | **RAII：编译器保证 _guard drop 时 span 退出，无泄漏风险** |

**关键洞察**：Rust 的所有权系统在这里转化为"追踪上下文的确定性管理"——没有 GC 延迟，没有手动关闭的遗忘风险。

### #[instrument]：编译期 AOP

```rust
#[instrument]
fn process_order(order_id: u64) {
    // 编译器自动插入：
    // let __span = info_span!("process_order", order_id = order_id);
    // let __guard = __span.enter();
}
```

**与 Java AOP 的本质区别**：

| 维度 | Java AOP (AspectJ/ByteBuddy) | Rust #[instrument] |
|------|-----------------------------|-------------------|
| 实现时机 | 运行时字节码注入 | 编译期宏展开 |
| 运行时开销 | 有（代理对象、反射） | **零**（直接插入代码） |
| 类型安全 | 运行时检查 | 编译期检查 |
| 调试难度 | 难以追踪（字节码已变） | 可用 `cargo expand` 查看展开结果 |

**设计意图**：将横切关注点（追踪插桩）在编译期解决，而不是运行时。这是 Rust "零成本抽象"哲学在可观测性领域的体现。

---

## 异步上下文传播：Rust 的方案

### 问题：异步代码的上下文地狱

在异步运行时中，一个 `async fn` 可能在**多个线程**上执行：

```rust
async fn handle_request() {
    do_something().await;  // 可能在 Thread A 执行到这里
    do_database().await;   // 挂起后，可能在 Thread B 恢复执行
}
```

### Java 的困境

Java 使用 **ThreadLocal** 存储当前 Span：

```java
// Java OpenTelemetry
Span span = tracer.spanBuilder("handle_request").startSpan();
try (Scope scope = span.makeCurrent()) {
    // ThreadLocal 存储当前 span
    doSomething();
} finally {
    span.end();
}
```

**ThreadLocal 在异步场景下的致命问题**：
1. 当 `Future` 在线程间迁移时，ThreadLocal 不会自动跟随
2. 线程池复用会导致**上下文污染**（新任务看到旧任务的 Span）
3. 解决方案（如 Project Reactor）：手动包装每个操作传递上下文，代码侵入性强

### Go 的方案

Go 通过 `context.Context` **显式传递**：

```go
func HandleRequest(ctx context.Context, req *Request) {
    ctx, span := tracer.Start(ctx, "handle_request")
    defer span.End()
    
    doSomething(ctx)  // 每个函数都要接收 ctx
}

func doSomething(ctx context.Context) {
    ctx, span := tracer.Start(ctx, "do_something")
    defer span.End()
    // ...
}
```

**问题**：每个函数签名都要包含 `ctx context.Context`，代码侵入性极强。

### Rust 的方案：Instrument trait

```rust
#[instrument]
async fn handle_request() {
    do_something().await;
    do_database().await;
}
```

**原理**：`.instrument(span)` 将 Span 绑定到 Future：

```rust
// 编译期展开后（简化）
async fn handle_request() {
    let __span = info_span!("handle_request");
    {
        let __guard = __span.enter();
        do_something().instrument(__span.clone()).await;
    }
    {
        let __guard = __span.enter();
        do_database().instrument(__span.clone()).await;
    }
}
```

**关键设计**：
- Span 作为 Future 状态机的一部分存储
- 当 Future 被 `poll` 时，自动 `enter`
- 当 `poll` 返回 `Pending`，自动 `exit`
- Future 在线程间迁移时，Span **跟随状态机一起移动**

**对比总结**：

| 语言 | 机制 | 线程安全 | 代码侵入性 |
|------|------|---------|-----------|
| Java | ThreadLocal + 手动包装 | 线程池污染风险 | 中（需使用框架包装） |
| Go | context.Context 显式传递 | 安全 | **高**（每个函数加 ctx 参数） |
| **Rust** | **Instrument trait 绑定 Future** | **安全** | **低**（#[instrument] 注解） |

---

## Layer 架构：组合优于继承

### 设计哲学

`tracing-subscriber` 使用 **Layer trait** 实现可组合的输出层。这是 Rust 社区"组合优于继承"设计哲学的典型案例。

```rust
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

// Layer A：输出到 stdout，人类可读格式
let fmt_layer = fmt::layer().with_ansi(true);

// Layer B：输出到文件，JSON 格式（供日志收集器解析）
let json_layer = fmt::layer().json().with_writer(make_writer);

// Layer C：发送到 OpenTelemetry（分布式追踪后端）
let otel_layer = tracing_opentelemetry::layer().with_tracer(jaeger_tracer);

// 独立过滤：每个 Layer 可以有自己的级别
let fmt_filter = EnvFilter::new("info");
let json_filter = EnvFilter::new("warn");
let otel_filter = EnvFilter::new("debug");

// 组合：registry + filter + layer
tracing_subscriber::registry()
    .with(fmt_layer.with_filter(fmt_filter))
    .with(json_layer.with_filter(json_filter))
    .with(otel_layer.with_filter(otel_filter))
    .init();
```

**每个 Layer 独立工作**：
- stdout Layer：只输出 `info` 及以上，带颜色，开发时使用
- JSON Layer：只输出 `warn` 及以上，结构化数据，生产环境收集
- OTel Layer：输出 `debug` 及以上，发送到 Jaeger，全链路追踪

### 与 Java/Go 的对比

| 维度 | Java Logback | Go slog | Rust tracing-subscriber |
|------|-------------|---------|------------------------|
| 架构模式 | 继承（Logger → Appender） | 单一 Handler | **组合（Layer 叠加）** |
| 多输出独立配置 | 复杂（需定义多个 Logger） | 需自定义 Handler | **简单（每个 Layer 独立 filter）** |
| 动态增删输出 | 不支持 | 不支持 | **支持（reload handle）** |
| 性能 | 中（同步锁） | 中 | **高（无锁设计，每个 Layer 独立通道）** |

---

## Rust Tracing 生态链路

### 核心 crate 层级

```
应用代码
    │
    ▼
tracing (应用层 API)
    │ 提供：span!、event!、#[instrument]
    ▼
tracing-subscriber (收集与分发)
    │ 提供：Registry、Layer trait、fmt::layer、EnvFilter
    ├── stdout / stderr（开发调试）
    ├── JSON 文件（日志收集）
    └── tracing-opentelemetry (桥接层)
              │
              ▼
        opentelemetry (OTel 标准实现)
              │
              ├── opentelemetry-jaeger ──→ Jaeger 后端
              ├── opentelemetry-zipkin ──→ Zipkin 后端
              ├── opentelemetry-otlp   ──→ OTLP 收集器
              └── opentelemetry-prometheus ──→ Prometheus 指标
```

### 各 crate 职责

| crate | 职责 | 类比（Java） |
|-------|------|------------|
| `tracing` | 应用层插桩 API | SLF4J API |
| `tracing-subscriber` | 订阅者实现和 Layer 组合 | Logback/Log4j2 |
| `tracing-opentelemetry` | 将 tracing 数据转换为 OTel 格式 | Brave (Zipkin) |
| `opentelemetry` | OpenTelemetry 标准实现 | OpenTelemetry Java SDK |
| `opentelemetry-jaeger` | 导出到 Jaeger | jaeger-client |

---

## 生产环境实践

### 1. JSON 输出 + 结构化字段

```rust
use tracing_subscriber::fmt;

let json_layer = fmt::layer()
    .json()
    .with_current_span(true)   // 包含当前 span 名称
    .with_span_list(true);     // 包含 span 层级路径
```

输出示例：
```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "INFO",
  "fields": { "message": "订单处理完成" },
  "target": "my_service::orders",
  "span": { "name": "process_order", "order_id": 42 },
  "spans": [
    { "name": "handle_request", "request_id": 123 },
    { "name": "process_order", "order_id": 42 }
  ]
}
```

### 2. 采样控制

高 QPS 服务不能记录所有请求的追踪数据，需要采样：

```rust
use opentelemetry::sdk::trace::Sampler;

let tracer = opentelemetry_jaeger::new_agent_pipeline()
    .with_service_name("my-service")
    .with_sampler(Sampler::Probability(0.01))  // 只采样 1%
    .install_simple()
    .expect("Error initializing Jaeger exporter");
```

### 3. 动态日志级别调整

```rust
use tracing_subscriber::{reload, EnvFilter};

let (filter, reload_handle) = reload::Layer::new(EnvFilter::new("info"));

tracing_subscriber::registry()
    .with(filter)
    .with(fmt::layer())
    .init();

// 运行时通过 HTTP API 调整级别
// reload_handle.reload(EnvFilter::new("debug,my_module=trace"))?;
```

### 4. 敏感数据脱敏

```rust
#[instrument(skip(password, token), fields(user_id = %user.id))]
fn authenticate(user: &User, password: &str, token: &str) -> Result<Auth, Error> {
    // password 和 token 不会被记录到 span 中
}
```

---

## 设计决策对比：Rust vs Java vs Go

### 链路追踪实现对比

| 维度 | Rust (tracing) | Java (OpenTelemetry) | Go (OpenTelemetry) |
|------|---------------|---------------------|-------------------|
| 插桩方式 | 编译期宏 (`#[instrument]`) | 运行时 Agent / 手动 | 手动代码 |
| 上下文传播 | Future 绑定（无锁） | ThreadLocal（有锁） | context.Context（显式） |
| 异步支持 | **原生** | 需手动管理 Scope | 原生但需传递 ctx |
| 运行时开销 | **零**（编译期展开） | 有（代理、反射） | 低（直接调用） |
| 输出组合 | Layer 组合（灵活） | Appender 继承（僵化） | Handler 单一 |
| 分布式后端 | Jaeger/Zipkin/OTLP | Jaeger/Zipkin/OTLP | Jaeger/Zipkin/OTLP |

### 各语言的权衡

| 场景 | 推荐 |
|------|------|
| 已有 Java Spring 微服务 | OpenTelemetry Java Agent（无侵入） |
| 新 Java 服务 | OpenTelemetry SDK + Micrometer |
| Go 微服务 | OpenTelemetry Go SDK（所有函数传 ctx） |
| **Rust 微服务** | **tracing + tracing-opentelemetry（零成本 + 异步原生）** |
| 超高性能网关 | Rust tracing（无分配、无锁） |

---

## 运行示例

```bash
cargo run -p distributed_tracing
```

设置日志级别：
```bash
RUST_LOG=debug cargo run -p distributed_tracing
```

---

## 设计哲学总结

### Rust tracing 的核心创新

1. **结构化诊断系统，不是日志库**
   - Span + Event 模型天然适合分布式追踪的树状结构
   - 避免了传统日志"平面文本 + 正则解析"的低效模式

2. **编译期插桩，零运行时开销**
   - `#[instrument]` 在编译期展开，无代理、无反射
   - 性能敏感路径（如网关、数据库中间件）也能安全使用

3. **RAII + 所有权 = 确定性的上下文管理**
   - Span 的生命周期由 Rust 的所有权系统保证
   - 无需手动 `span.end()`，无上下文泄漏风险

4. **异步原生，无需 ThreadLocal**
   - `Instrument` trait 将 Span 绑定到 Future 状态机
   - 任务在线程间迁移时，上下文自动跟随

5. **组合式架构（Layer）**
   - 多个输出目标独立配置、独立过滤
   - 比继承体系更灵活，比单一 Handler 更强大

### 一句话总结

> **Java 的追踪是在运行时"包裹"代码，Go 的追踪是在调用链上"传递"上下文，Rust 的追踪是在编译期"编织"到代码中——然后利用所有权系统确保它永远不会出错。**
