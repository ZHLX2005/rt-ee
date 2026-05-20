# Rust 枚举、异常与错误处理方案

## 设计背景与问题域

Rust 的错误处理不是"另一种异常机制"，而是**完全摒弃了异常模型**，用类型系统重新构建的。理解这一点需要回答三个核心问题：

1. **Rust 枚举在错误处理中扮演什么角色？**
   - `Result<T, E>` 和 `Option<T>` 本身就是枚举
   - 自定义错误类型几乎总是枚举
   - 枚举的穷尽匹配让错误处理分支不可遗漏

2. **Rust 有"异常"吗？**
   - `panic!` 不是异常，而是**进程终止的渐进式表达**
   - Java 的异常是"可恢复的跳转"，Rust 的 panic 是"不可恢复的状态破坏"
   - `catch_unwind` 是隔离边界，不是常规控制流

3. **工程实践中如何选择处理方案？**
   - 库代码（Library）：精确错误类型，`thiserror` 生成 `Display`/`Error`
   - 应用代码（Application）：抹平错误类型，`anyhow` 提供上下文
   -  FFI/插件边界：`catch_unwind` 防止 panic 越界

---

## Rust 没有异常：panic 的本质

### panic 与 Java 异常的根本区别

| 维度 | Java Exception | Rust panic |
|------|---------------|------------|
| 设计意图 | 正常的错误传播机制 | 表示程序存在 bug |
| 恢复预期 | 调用者应该尝试恢复 | 通常不应恢复 |
| 控制流 | 栈展开（stack unwinding） | 默认也栈展开，但可配置为直接 abort |
| 类型系统 | 绕过类型系统 | 与类型系统无关 |
| 性能成本 | 创建异常对象昂贵 | panic 路径不返回，成本不敏感 |

```rust
// 这是错误的 Rust 代码风格：把 panic 当异常用
fn parse_id(s: &str) -> u32 {
    s.parse().expect("invalid id") // 错误！用户输入不应该 panic
}

// 正确的做法：返回 Result
fn parse_id(s: &str) -> Result<u32, std::num::ParseIntError> {
    s.parse() // 失败是预期内的，调用者决定如何处理
}
```

### panic 的两种策略

```toml
# Cargo.toml
[profile.release]
panic = "abort"  # 直接终止进程，不栈展开。更小的二进制，更快的 panic
# panic = "unwind" # 默认：栈展开，允许 catch_unwind
```

**何时使用 unwind**：
- 需要 `catch_unwind` 隔离 panic（如线程边界、FFI 边界）
- 需要析构函数在 panic 时执行（RAII 资源清理）

**何时使用 abort**：
- 嵌入式环境，无栈展开支持
- 追求最小二进制体积
- 认为 panic 即不可恢复，不需要清理

### catch_unwind：隔离而非处理

```rust
use std::panic;

fn main() {
    let result = panic::catch_unwind(|| {
        panic!("something went wrong");
    });

    match result {
        Ok(_) => println!("Success"),
        Err(_) => println!("Caught panic"), // 仅用于隔离，不用于业务逻辑
    }
}
```

**关键原则**：`catch_unwind` 是**边界防护设施**（如防止子任务 panic 拖垮整个线程池），不是**业务错误处理机制**。

---

## 枚举作为错误类型：从基础到工程化

### 为什么自定义错误应该是枚举？

Rust 的错误类型需要实现 `std::error::Error` trait。枚举天然适合：每个变体代表一种错误情况，且可以携带不同的上下文数据。

```rust
use std::fmt;
use std::io;

// 手写的错误枚举：需要手动实现 Display 和 Error
#[derive(Debug)]
enum AppError {
    Io(io::Error),
    Parse(std::num::ParseIntError),
    Config { key: String, reason: String },
    NotFound(u64),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Io(e) => write!(f, "IO error: {}", e),
            AppError::Parse(e) => write!(f, "Parse error: {}", e),
            AppError::Config { key, reason } => {
                write!(f, "Config error for key '{}': {}", key, reason)
            }
            AppError::NotFound(id) => write!(f, "Resource {} not found", id),
        }
    }
}

impl std::error::Error for AppError {
    fn source(&self) -> Option<&( dyn std::error::Error + 'static )> {
        match self {
            AppError::Io(e) => Some(e),
            AppError::Parse(e) => Some(e),
            _ => None,
        }
    }
}
```

### thiserror：让枚举错误类型自动化

手写 `Display` 和 `Error` 实现很繁琐。`thiserror` 用 derive 宏自动生成：

```rust
use thiserror::Error;

#[derive(Error, Debug)]
enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),  // #[from] 自动生成 From<io::Error>

    #[error("Parse error: {0}")]
    Parse(#[from] std::num::ParseIntError),

    #[error("Config error for key '{key}': {reason}")]
    Config { key: String, reason: String },

    #[error("Resource {0} not found")]
    NotFound(u64),
}
```

**`thiserror` 生成的代码等价于上面手写的 50+ 行**。

### #[from] 的魔法：自动错误转换

```rust
// 以下代码为什么能编译？
fn read_and_parse(path: &str) -> Result<i32, AppError> {
    let content = std::fs::read_to_string(path)?; // io::Error -> AppError
    let num: i32 = content.trim().parse()?;        // ParseIntError -> AppError
    Ok(num)
}

// 因为 #[from] 让 thiserror 自动生成了：
// impl From<io::Error> for AppError { ... }
// impl From<ParseIntError> for AppError { ... }
// ? 运算符内部调用 Into::into()，即 From::from()
```

### 非穷尽枚举（Non-exhaustive）：库的错误扩展性

```rust
// 在库代码中标记枚举为非穷尽
#[non_exhaustive]
pub enum ApiError {
    Network,
    Timeout,
    // 未来可以安全地添加新变体，不会破坏用户代码的 match
}

// 用户代码必须包含通配分支
match err {
    ApiError::Network => ...,
    ApiError::Timeout => ...,
    _ => ..., // 必须处理未来可能新增的变体
}
```

---

## 错误处理的分层架构

### 库层：精确错误类型（thiserror）

库的消费者需要知道**具体出了什么问题**，以便做出不同的响应。

```rust
// 库的公共 API
use thiserror::Error;

#[derive(Error, Debug)]
pub enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Query timeout after {0}ms")]
    Timeout(u64),

    #[error("Record not found: {0}")]
    NotFound(String),
}

pub fn query(sql: &str) -> Result<Vec<Row>, DatabaseError> {
    // ...
}
```

### 应用层：抹平错误类型（anyhow）

应用通常只需要**报告错误并退出**或**返回 500**，不需要区分每一种错误。

```rust
use anyhow::{Context, Result};

fn main() -> Result<()> {
    let config = std::fs::read_to_string("app.toml")
        .with_context(|| "Failed to read config file")?;

    let db = connect_db(&config)
        .context("Failed to connect to database")?;

    let users = db.query("SELECT * FROM users")
        .context("Failed to fetch users")?;

    println!("{:?}", users);
    Ok(())
}
```

**`anyhow::Result<T>` 等价于 `Result<T, anyhow::Error>`**，可以容纳任何实现了 `std::error::Error` 的错误，并自动维护错误链。

### 对比：thiserror vs anyhow

| 维度 | thiserror | anyhow |
|------|-----------|--------|
| 用途 | 定义错误类型 | 消费错误类型 |
| 使用位置 | 库（library） | 应用（application） |
| 错误类型 | 精确、结构化 | 动态、擦除 |
| 上下文 | 通过枚举变体 | 通过 `.context()` |
| 是否实现 Error | 是 | 是（包装任意 Error） |

---

## ? 运算符与 Try trait：深入原理

### ? 运算符的本质

```rust
// expr? 等价于：
match expr {
    Ok(v) => v,
    Err(e) => return Err(From::from(e)), // 自动类型转换！
}
```

这就是为什么 `io::Error` 可以自动变成 `AppError` —— `?` 内部调用了 `From::from()`。

### 自定义 Result 类型别名

```rust
// 为特定领域定义 Result 别名，简化签名
pub type DbResult<T> = Result<T, DatabaseError>;

pub fn find_user(id: u64) -> DbResult<User> {
    // ...
}
```

### Option 与 Result 的互操作

```rust
fn find_user_score(id: u64, db: &Db) -> Result<i32, &'static str> {
    let user = db.find(id).ok_or("User not found")?; // Option -> Result
    let score = user.score?;                           // Option -> Result
    Ok(score)
}
```

---

## 错误链与上下文：从"什么错了"到"为什么错了"

### anyhow 的上下文链

```rust
use anyhow::{Context, Result};

fn process_file(path: &str) -> Result<Vec<Record>> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read file: {}", path))?;

    let records = content.lines()
        .map(|line| parse_record(line))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse records")?;

    Ok(records)
}

// 错误输出：
// Failed to parse records
// Caused by:
//   0: Failed to parse line: "invalid,data"
//   1: missing field `score`
```

### 手动实现错误链（不用 anyhow）

```rust
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct ContextError {
    msg: String,
    source: Box<dyn Error + Send + Sync>,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl Error for ContextError {
    fn source(&self) -> Option<&( dyn Error + 'static )> {
        Some(self.source.as_ref())
    }
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 错误表示 | `Result<T, E>`（枚举） | Exception（类层次结构） | `(T, error)` 多返回值 |
| 空值表示 | `Option<T>`（枚举） | `null` | `nil` |
| 强制处理 | 编译器强制（must_use） | Checked 强制，Unchecked 不强制 | 不强制 |
| 传播语法 | `?` 运算符 + `From` | `throws` + 栈展开 | 手动 `if err != nil` |
| 错误类型定义 | 枚举 + derive | 类继承 | `error` interface |
| 上下文链 | `anyhow::Context` | Exception cause | `fmt.Errorf("%w")` |
| "异常" | `panic!`（不可恢复） | `Exception`（可恢复） | `panic`（不可恢复） |

---

## 运行示例

```bash
cargo run -p error_handling_patterns
```
