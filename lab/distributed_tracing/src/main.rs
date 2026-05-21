// Rust 链路追踪生态：设计意图深度解析
//
// 核心问题：在分布式系统中，如何低成本地追踪请求的完整链路？
//
// 为什么不是 log crate？
// - log 是平面的：每个日志行独立，没有因果关系
// - tracing 是有结构的：Span 建立时间上的因果关系，Event 标记时间点
//
// 为什么 Rust 的 tracing 设计特别？
// - 零成本抽象：Span 的进入/退出利用 RAII（Drop trait），无运行时 GC
// - 类型安全：#[instrument] 在编译期插桩，错误在编译时发现
// - 异步原生：不需要 ThreadLocal（Java 的痛点），通过 Subscriber 内部状态管理
//
// 对比 Java：
// - Java OpenTelemetry：依赖 ThreadLocal 存储 SpanContext，线程池场景容易泄漏
// - Java Brave/Sleuth：基于拦截器，对代码有侵入性
// - Rust tracing：编译期插桩 + Subscriber 层解耦，无 ThreadLocal 开销
//
// 对比 Go：
// - Go：trace 信息通过 context.Context 显式传递，每个函数都要接收 ctx 参数
// - Rust：通过 Span 的 enter/exit 隐式管理，配合 async 自动传播

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, info_span, instrument, Instrument};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, fmt};

// === 1. 为什么 tracing 不是"更好的日志" ===
//
// log crate 的设计是平面的：
//   info!("请求开始");
//   info!("查询数据库");
//   info!("请求完成");
// 这三行之间没有任何关系，在日志系统中是孤立的。
//
// tracing 的设计是有结构的：
//   span("handle_request") {    // 一段有上下文的"时间范围"
//     event("请求开始");
//     span("db_query") {
//       event("查询数据库");
//     }
//     event("请求完成");
//   }
//
// 关键洞察：Span 是"带有因果关系的上下文容器"，Event 是"在该上下文内发生的事情"
// 这种树状结构天然适合分布式追踪的链路表示

fn demo_span_vs_log() {
    println!("\n=== 1. Span vs Log ===");

    // 创建一个 span：代表"处理订单"这个操作的时间范围
    let order_span = info_span!("process_order", order_id = 42, user_id = 100);

    // enter() 返回一个守卫，当守卫 drop 时 span 自动退出
    // 这是 RAII 模式在 tracing 中的应用——Rust 的所有权系统保证 span 正确关闭
    let _guard = order_span.enter();

    info!("订单处理开始");

    {
        // 子 span：数据库查询
        let db_span = info_span!("db_query", table = "orders", duration_ms = 23);
        let _db_guard = db_span.enter();
        info!("执行 SQL: SELECT * FROM orders WHERE id = 42");
    } // db_span 在这里结束，持续时间被记录

    info!("订单处理完成，状态: success");
} // order_span 在这里结束

// === 2. #[instrument]：编译期自动插桩 ===
//
// 设计意图：减少样板代码，同时保持类型安全
// Java 等价物：@Traceable + AOP（运行时字节码注入）
// Rust 方案：过程宏在编译期展开，零运行时开销
//
// instrument 宏会自动：
// 1. 创建以函数名命名的 span
// 2. 将函数参数作为 span 字段
// 3. 在函数入口 enter，在函数出口 exit（通过 Drop）

#[instrument]
fn calculate_discount(price: f64, user_level: u32) -> f64 {
    info!(%price, %user_level, "计算折扣");
    let discount = match user_level {
        0 => 0.0,
        1..=3 => price * 0.05,
        4..=6 => price * 0.10,
        _ => price * 0.20,
    };
    info!(%discount, "折扣计算完成");
    discount
}

// skip 某些字段：避免大对象或大容量数据被记录到 span 中
// 这在处理敏感数据（如密码、token）时尤为重要
#[instrument(skip(password), fields(auth_method = "token"))]
fn authenticate_user(username: &str, password: &str) -> bool {
    info!(%username, "用户认证");
    // password 不会被记录到 span 中（安全考虑）
    password.len() > 6
}

// === 3. 异步上下文传播 ===
//
// 核心挑战：async/await 中，函数可能在不同线程上恢复执行
// Java 方案：使用 ThreadLocal，但线程切换时上下文会丢失
//   - ThreadLocal 在异步场景下的问题：task 在线程间迁移时，ThreadLocal 不会自动跟随
//   - 解决方案（如 Reactor）：手动包装 Callable/Runnable 传递上下文
// Go 方案：context.Context 显式传递，每个函数签名都要接收 ctx 参数
//   - func HandleRequest(ctx context.Context, req *Request) { ... }
//   - 所有调用链上的函数都需要 ctx，侵入性强
// Rust 方案：.instrument(span) 将 Future 绑定到 Span
//   - 当 Future 被 poll 时，span 自动 enter
//   - 当 poll 返回 Pending，span 自动 exit
//   - task 在线程间迁移时，span 状态跟随 Future 本身
//
// 关键设计：Instrument trait 将 Span 附加到 Future
// 这利用了 Rust 的 Pin 保证和异步状态机——状态机变换时上下文自动切换

#[instrument]
async fn fetch_user_async(user_id: u64) -> String {
    info!("开始异步获取用户");
    sleep(Duration::from_millis(10)).await;
    info!("异步获取完成");
    format!("User({})", user_id)
}

#[instrument]
async fn process_order_async(order_id: u64) -> Result<String, &'static str> {
    info!("开始异步处理订单");

    // .instrument() 将当前 span 绑定到异步操作
    // 当 await 挂起和恢复时，span 的 enter/exit 自动管理
    let user = fetch_user_async(order_id).await;

    sleep(Duration::from_millis(5)).await;

    info!(%user, "订单处理完成");
    Ok(user)
}

// 显式使用 Instrument trait（与 #[instrument] 自动处理形成对比）
async fn manual_instrument_demo() {
    println!("\n=== 3b. 显式 Instrument ===");

    let span = info_span!("manual_task", task_id = 99);

    // 使用 .instrument() 显式将 Future 绑定到 Span
    // 这在需要精细控制哪些异步操作被追踪时很有用
    let future = async {
        info!("手动 instrument 的异步任务");
        sleep(Duration::from_millis(5)).await;
    };

    future.instrument(span).await;
}

// === 4. Layer 架构：组合式订阅者 ===
//
// 设计哲学：组合优于继承
// tracing-subscriber 使用 Layer trait 实现可组合的输出层
// 每个 Layer 负责一种输出格式或目标，可以叠加使用
//
// 对比：
// - Java Logback/Log4j2：Appender 是继承体系，配置复杂
//   - 一个 Logger 只能有一个级别，多个 Appender 共享同一级别
// - Go slog：Handler 接口，组合能力有限
//   - 一个 Handler 处理所有输出，难以独立配置
// - Rust tracing-subscriber：Layer 可以任意组合，每个 Layer 独立过滤和格式化
//   - Layer A: stdout + info 级别
//   - Layer B: JSON 文件 + debug 级别
//   - Layer C: OpenTelemetry + warn 级别
//   - 三者独立工作，互不干扰

fn init_tracing() {
    // 格式化层：输出到 stdout，带颜色
    let fmt_layer = fmt::layer()
        .with_target(true)      // 显示目标模块
        .with_thread_ids(true)  // 显示线程 ID（对异步调试很重要）
        .with_line_number(true) // 显示行号
        .with_ansi(true);       // ANSI 颜色

    // 环境过滤层：从 RUST_LOG 环境变量读取过滤规则
    // 例如：RUST_LOG=info,my_module=debug
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // 组合：registry 作为核心，叠加 filter 和 fmt_layer
    // 这是"装饰器模式"在 Rust 中的实现
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .init();
}

// === 5. 模拟分布式追踪：Trace ID 传播 ===
//
// 在微服务架构中，请求从一个服务传播到另一个服务
// 需要传递 trace_id（全局唯一，标识整个请求链路）
// 和 span_id（当前 span 唯一）
//
// 真实场景（OpenTelemetry + tracing-opentelemetry）：
// - 服务 A 生成 trace_id，创建 root span
// - HTTP 请求头携带 traceparent: 00-{trace_id}-{span_id}-01
// - 服务 B 解析 traceparent，创建 child span（parent_id = span_id）
// - 所有 span 发送到 Jaeger/Zipkin，形成完整链路图
//
// Rust tracing 生态链路：
// tracing (应用层插桩)
//   -> tracing-subscriber (收集和分发)
//   -> tracing-opentelemetry (转换为 OTel 格式)
//   -> opentelemetry-jaeger / opentelemetry-otlp (导出到后端)
//
// 这里用 tracing 的字段模拟核心机制

#[derive(Debug)]
struct RequestContext {
    trace_id: String,
    #[allow(dead_code)]
    parent_span_id: Option<String>,
}

#[instrument(
    fields(
        trace_id = %ctx.trace_id,
        service = "service-a"
    ),
    skip(ctx)
)]
async fn service_a_handler(ctx: RequestContext) {
    info!("Service A 处理请求");

    // 模拟调用 Service B
    let child_ctx = RequestContext {
        trace_id: ctx.trace_id.clone(),
        parent_span_id: Some("span-a-1".to_string()),
    };
    service_b_handler(child_ctx).await;
}

#[instrument(
    fields(
        trace_id = %ctx.trace_id,
        service = "service-b"
    ),
    skip(ctx)
)]
async fn service_b_handler(ctx: RequestContext) {
    info!("Service B 处理请求");

    // 模拟数据库操作
    db_query(&ctx).await;
}

#[instrument(
    fields(
        trace_id = %ctx.trace_id,
        db.table = "users"
    ),
    skip(ctx)
)]
async fn db_query(ctx: &RequestContext) {
    info!("数据库查询");
}

// === 6. 生产环境设计模式 ===
//
// 1. JSON 输出（便于日志收集器如 ELK/Loki 解析）
//    let json_layer = fmt::layer().json().with_current_span(true);
//
// 2. 采样控制：避免高流量下追踪数据爆炸
//    tracing-opentelemetry 支持概率采样（如只记录 1% 的请求）
//
// 3. 动态级别调整：无需重启修改日志级别
//    tracing-subscriber 支持 reload handle
//    let (filter, reload_handle) = reload::Layer::new(EnvFilter::new("info"));
//    // 运行时通过 HTTP API 调用 reload_handle.reload(new_filter)
//
// 4. 性能优化：
//    - 使用 tracing::span::Span::none() 创建无操作 span（零开销）
//    - 使用静态 span（Span::current()）避免重复创建
//    - 在高频路径使用 tracing::Level::TRACE 并配合采样

fn demo_production_patterns() {
    println!("\n=== 6. 生产环境设计模式 ===");
    println!("
// JSON 输出层（用于 ELK/Loki 收集）
let json_layer = fmt::layer()
    .json()
    .with_current_span(true)
    .with_span_list(true);

// 采样控制：只记录 1% 的请求
// tracing-opentelemetry 提供概率采样器
let otel_layer = tracing_opentelemetry::layer()
    .with_tracer(jaeger_tracer)
    .with_sampler(Sampler::Probability(0.01));

// 动态级别：通过 HTTP API 或配置文件调整
// tracing-subscriber 支持 reload handle
let (filter, reload_handle) = reload::Layer::new(EnvFilter::new(\"info\"));

// 组合输出
registry()
    .with(filter)
    .with(json_layer)
    .with(otel_layer)
    .init();
    ");
}

#[tokio::main]
async fn main() {
    init_tracing();

    demo_span_vs_log();
    calculate_discount(100.0, 5);
    authenticate_user("alice", "secret123");

    let _ = process_order_async(42).await;

    manual_instrument_demo().await;

    let ctx = RequestContext {
        trace_id: "abc123def456".to_string(),
        parent_span_id: None,
    };
    service_a_handler(ctx).await;

    demo_production_patterns();

    println!("\n=== 关键洞察 ===");
    println!("1. tracing 不是日志库，而是'带有因果关系的诊断系统'");
    println!("2. Span + RAII = 零成本、确定性的上下文管理");
    println!("3. #[instrument] = 编译期 AOP，无运行时字节码注入开销");
    println!("4. Layer 架构 = 组合式输出，比继承体系更灵活");
    println!("5. 异步原生设计 = 无需 ThreadLocal，跨线程上下文安全传递");
    println!("6. 与 OpenTelemetry 生态无缝集成，支持 Jaeger/Zipkin/Prometheus");
}
