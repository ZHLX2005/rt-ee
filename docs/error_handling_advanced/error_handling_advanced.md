# thiserror/anyhow 工程实践进阶

## 设计背景与问题域

`thiserror` 和 `anyhow` 是 Rust 错误处理生态的事实标准，但许多开发者只停留在"库用 thiserror，应用用 anyhow"的表层认知。真实工程中的问题远比这复杂：

1. **库的错误类型如何设计才能既精确又稳定？** 新增错误变体不能破坏下游编译。
2. **`#[from]` 和 `#[source]` 有什么区别？** 自动转换 vs 错误链透视。
3. **anyhow 的 `.context()` 应该加在哪一层？** 过度包装会让错误链失去价值。
4. **动态分发（anyhow）有没有性能代价？** 热路径是否应该避免 `Box<dyn Error>`？
5. **异步代码中的错误传播有什么特殊考量？**

本文聚焦这些工程实践细节。

---

## thiserror 的工程实践

### 1. #[from] vs #[source]：自动转换与错误链

```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DataError {
    // #[from] 做两件事：
    // 1. 生成 impl From<std::io::Error> for DataError
    // 2. 自动将 io::Error 标记为 #[source]
    #[error("IO failed: {0}")]
    Io(#[from] std::io::Error),

    // 只有 #[source]，没有 #[from]
    // 意味着 ? 运算符不会自动转换 serde_json::Error 为 DataError
    // 需要手动 .map_err() 或额外写一个 #[from] 变体
    #[error("JSON invalid at path: {path}")]
    Json {
        path: String,
        #[source]
        source: serde_json::Error,
    },
}
```

**工程规则**：
- 第三方基础设施错误（io、网络）用 `#[from]`，调用方可以直接 `?`
- 业务语义错误（JSON 解析但路径相关）用 `#[source]` + 手动构造，强制调用方提供上下文

### 2. #[error(transparent)]：透明透传

```rust
#[derive(Error, Debug)]
pub enum ServiceError {
    #[error("database error")]
    Db(#[from] DbError),

    // transparent：Display 完全透传底层错误，不添加额外前缀
    // 适合：包装一个底层错误但不想改变其消息的场景
    #[error(transparent)]
    Config(#[from] ConfigError),
}
```

### 3. #[non_exhaustive]：库错误的版本兼容性

```rust
// 库的公开错误类型
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum ApiError {
    #[error("not found")]
    NotFound,
    #[error("unauthorized")]
    Unauthorized,
    #[error("internal error")]
    Internal,
}

// 下游代码必须写通配分支，未来新增变体不会导致编译失败
match err {
    ApiError::NotFound => StatusCode::NOT_FOUND,
    ApiError::Unauthorized => StatusCode::UNAUTHORIZED,
    ApiError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
    _ => StatusCode::INTERNAL_SERVER_ERROR, // 必须存在
}
```

### 4. 自定义 Result 类型别名与类型安全

```rust
// 在库的 lib.rs 中定义
pub type Result<T> = std::result::Result<T, ApiError>;

// 使用
pub fn fetch_user(id: u64) -> Result<User> {
    // ...
}
```

**陷阱**：如果多个库都定义自己的 `Result`，同时引入会冲突。建议命名：`ApiResult`、`DbResult`。

### 5. Backtrace 的按需捕获

```rust
use std::backtrace::Backtrace;

#[derive(Error, Debug)]
pub enum CriticalError {
    #[error("critical failure: {message}")]
    Failure {
        message: String,
        // 仅在 RUST_BACKTRACE=1 时自动填充
        backtrace: Backtrace,
    },
}
```

---

## anyhow 的工程实践

### 1. Context 的粒度：何时添加、何时停止

**反模式：过度包装**

```rust
// 错误示范：每一层都加无意义的 context
let data = read_file(path)
    .context("read file")?           // 第一层
    .parse()
    .context("parse")?               // 第二层
    .validate()
    .context("validate")?;           // 第三层
// 错误链："validate" -> "parse" -> "read file" -> "No such file"
// 每一层都只有动词，没有业务语义
```

**正确做法：context 应该包含业务语义**

```rust
let config = std::fs::read_to_string(path)
    .with_context(|| format!("Failed to load config from {}", path))?;

let settings: Settings = toml::from_str(&config)
    .with_context(|| "Config file contains invalid TOML")?;

settings.validate()
    .with_context(|| "Config validation failed")?;
```

**工程规则**：
- **边界处加 context**：跨模块/跨服务的调用边界
- **异常处加 context**：知道"做了什么"导致失败时
- **不要重复**：如果底层已经有好的错误消息，不要重新包装

### 2. anyhow! 宏：即时构造错误

```rust
use anyhow::{anyhow, Result};

fn divide(a: f64, b: f64) -> Result<f64> {
    if b == 0.0 {
        return Err(anyhow!("division by zero: {} / {}", a, b));
    }
    Ok(a / b)
}
```

### 3. 向下转型（Downcast）：从 anyhow 中提取具体错误

```rust
use anyhow::Result;

fn main() -> Result<()> {
    let result = operation();

    if let Err(e) = result {
        // 尝试从 anyhow::Error 中提取底层错误
        if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
            if io_err.kind() == std::io::ErrorKind::NotFound {
                println!("File not found, using default");
                return Ok(());
            }
        }
        return Err(e);
    }

    Ok(())
}
```

**注意**：downcast 破坏了 anyhow 的抽象，应该限制在极少数场景（如根据错误类型做重试策略）。

### 4. {:#} 与 {}：错误链的两种打印格式

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let err = inner().context("outer context").unwrap_err();

    println!("Display: {}", err);
    // outer context

    println!("Debug: {:?}", err);
    // outer context
    // Caused by:
    //   inner error

    println!("Alternate Display: {:#}", err);
    // outer context
    // Caused by:
    //   inner error
}
```

---

## 混合模式：从 thiserror 到 anyhow 的转换

### 典型分层架构

```
library crate (thiserror)
    ↓ Result<T, LibraryError>
application crate (anyhow)
    ↓ .context() / .map_err()
HTTP handler
    ↓ 映射为 HTTP 状态码
```

### 转换策略

```rust
use anyhow::{Context, Result};
use my_library::{ApiError, Client};

async fn handle_request(client: &Client, id: u64) -> Result<UserDto> {
    // 策略 1：直接 ?（需要 LibraryError 实现 Into<anyhow::Error>）
    // 因为 anyhow::Error 可以从任何 Error + Send + Sync + 'static 构造
    let user = client.fetch_user(id).await?;

    // 策略 2：添加业务上下文
    let profile = client
        .fetch_profile(id)
        .await
        .with_context(|| format!("Failed to fetch profile for user {}", id))?;

    // 策略 3：映射为业务结果（不传播错误）
    match client.fetch_settings(id).await {
        Ok(settings) => Ok(UserDto::from((user, profile, settings))),
        Err(ApiError::NotFound) => Ok(UserDto::from((user, profile, Default::default()))),
        Err(e) => Err(e.into()), // 其他错误继续传播
    }
}
```

---

## HTTP API 中的错误处理模式

### 从 anyhow 到 HTTP 响应的映射

```rust
use axum::{
    response::{IntoResponse, Response},
    http::StatusCode,
};
use anyhow::Error;

// 定义一个包装类型，将 anyhow::Error 转为 HTTP 响应
struct AppError(Error);

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        // 日志记录完整错误链（包含根因）
        tracing::error!("Request failed: {:#}", self.0);

        // 客户端只收到通用消息，不暴露内部细节
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
    }
}

// 自动转换：anyhow::Error -> AppError
impl<E> From<E> for AppError where E: Into<Error> {
    fn from(err: E) -> Self {
        Self(err.into())
    }
}
```

### 分层暴露策略

```rust
// 内部服务错误：记录完整堆栈和上下文
// 客户端响应：根据错误类型返回不同状态码，但消息泛化

match err {
    ApiError::NotFound => (StatusCode::NOT_FOUND, "Resource not found"),
    ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized"),
    ApiError::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "Rate limited"),
    _ => {
        tracing::error!("Internal error: {:#}", err);
        (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
    }
}
```

---

## 性能考量：anyhow 的动态分发成本

### anyhow 的内部结构

```rust
// anyhow::Error 本质上是：
struct Error {
    inner: Box<DynError>,  // 堆分配 + 动态分发
}
```

**成本**：
- 构造：一次堆分配（`Box::new`）
- 传播：`?` 运算符移动指针，无额外分配
- 打印：动态分发调用 `Display::fmt`

### 何时应该避免 anyhow

| 场景 | 建议 | 原因 |
|------|------|------|
| 库公开 API | 用 thiserror | 调用方需要精确匹配错误类型 |
| 高频热路径（>10k/s）| 用 thiserror 或 `Result<T, SmallError>` | 避免堆分配 |
| CLI/HTTP 应用 | 用 anyhow | 错误处理频率低，开发效率优先 |
| 嵌入式/实时系统 | 用静态错误枚举 | 禁止堆分配 |

### thiserror 的零成本保证

```rust
#[derive(thiserror::Error, Debug)]
pub enum MyError {
    #[error("io")]
    Io(#[from] std::io::Error),
}

// thiserror 生成的代码完全内联，等价于手写实现
// 没有运行时开销，没有额外分配
```

---

## 异步错误处理

### async fn 中的 ? 运算符

```rust
use anyhow::{Context, Result};

async fn fetch_data(url: &str) -> Result<String> {
    let resp = reqwest::get(url)
        .await
        .with_context(|| format!("Failed to GET {}", url))?;

    let body = resp.text()
        .await
        .context("Failed to read response body")?;

    Ok(body)
}
```

### Stream/Iterator 中的错误聚合

```rust
use anyhow::Result;
use futures::stream::{self, StreamExt};

async fn fetch_all(urls: Vec<String>) -> Result<Vec<String>> {
    let results: Vec<Result<String>> = stream::iter(urls)
        .map(|url| async move { fetch_data(&url).await })
        .buffer_unordered(10)
        .collect()
        .await;

    // 收集所有错误，或返回第一个错误
    results.into_iter().collect::<Result<Vec<_>>>()
}
```

---

## 运行示例

```bash
cargo run -p error_handling_advanced
```
