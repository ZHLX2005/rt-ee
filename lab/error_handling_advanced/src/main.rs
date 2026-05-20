// thiserror/anyhow 工程实践进阶
//
// 演示场景：
// 1. 库层用 thiserror 定义精确错误类型
// 2. 应用层用 anyhow 消费错误并添加上下文
// 3. #[from] vs #[source] 的区别
// 4. 错误链的构建和打印
// 5. 向下转型（downcast）提取具体错误
// 6. anyhow! 宏构造即时错误

use anyhow::{anyhow, Context, Result as AnyhowResult};
use std::error::Error;
use thiserror::Error;

// === 库层：用 thiserror 定义精确错误类型 ===

/// 库的错误类型：精确、结构化、带版本兼容性保护
#[derive(Error, Debug)]
#[non_exhaustive]
pub enum LibraryError {
    // #[from] 自动生成 From<std::io::Error>，支持 ? 自动转换
    #[error("IO failed: {0}")]
    Io(#[from] std::io::Error),

    // #[source] 标记底层错误，但不生成 From（需要手动构造）
    #[error("JSON parse failed at path '{path}'")]
    JsonParse {
        path: String,
        #[source]
        source: serde_json::Error,
    },

    // 纯业务错误，无底层原因
    #[error("Validation failed: field '{field}' has invalid value '{value}'")]
    Validation { field: String, value: String },

    #[error("Resource not found: {0}")]
    NotFound(String),

    // transparent：直接透传底层错误的 Display
    #[error(transparent)]
    Config(#[from] ConfigError),
}

#[derive(Error, Debug)]
#[error("Configuration error: {message}")]
pub struct ConfigError {
    message: String,
}

// 库提供自定义 Result 别名
pub type LibraryResult<T> = std::result::Result<T, LibraryError>;

// === 库的公开 API ===

pub struct Database;

impl Database {
    pub fn connect(conn_str: &str) -> LibraryResult<Self> {
        if conn_str.is_empty() {
            return Err(ConfigError {
                message: "Connection string is empty".into(),
            }.into());
        }
        if !conn_str.starts_with("postgres://") {
            return Err(LibraryError::Validation {
                field: "conn_str".into(),
                value: conn_str.into(),
            });
        }
        Ok(Database)
    }

    pub fn query(&self, sql: &str) -> LibraryResult<Vec<String>> {
        if sql.contains("DROP") {
            return Err(LibraryError::NotFound("table".into()));
        }
        // 模拟 IO 错误：? 会自动将 io::Error 转为 LibraryError::Io
        // std::fs::read_to_string("/nonexistent")?;
        Ok(vec![
            format!("Result of: {}", sql),
            "Alice".into(),
            "Bob".into(),
        ])
    }

    pub fn parse_config(json: &str, path: &str) -> LibraryResult<serde_json::Value> {
        let value = serde_json::from_str(json).map_err(|e| LibraryError::JsonParse {
            path: path.into(),
            source: e,
        })?;
        Ok(value)
    }
}

// === 应用层：用 anyhow 消费库错误 ===

fn run_app() -> AnyhowResult<()> {
    println!("--- 场景 1：正常流程 ---");
    let db = Database::connect("postgres://localhost/mydb")
        .context("Failed to initialize database connection")?;

    let users = db
        .query("SELECT * FROM users")
        .context("Failed to fetch user list")?;
    println!("Users: {:?}", users);

    println!("\n--- 场景 2：验证错误（带业务上下文）---");
    if let Err(e) = Database::connect("mysql://localhost/mydb") {
        // 这里将 LibraryError 转为 anyhow::Error，并添加应用层上下文
        let wrapped: anyhow::Error = e.into();
        println!("Caught library error: {}", wrapped);
    }

    println!("\n--- 场景 3：JSON 解析错误（带 #[source] 链） ---");
    if let Err(e) = Database::parse_config("not json", "config.json") {
        println!("Library error: {}", e);
        // 遍历错误链
        let mut source = e.source();
        while let Some(s) = source {
            println!("  Caused by: {}", s);
            source = s.source();
        }
    }

    println!("\n--- 场景 4：anyhow! 宏构造即时错误 ---");
    let result = divide(10.0, 0.0);
    if let Err(e) = result {
        println!("anyhow error: {}", e);
    }

    println!("\n--- 场景 5：向下转型提取具体错误 ---");
    match simulate_io_operation(false) {
        Ok(_) => println!("IO succeeded"),
        Err(e) => {
            if let Some(io_err) = e.downcast_ref::<std::io::Error>() {
                println!("Detected IO error kind: {:?}", io_err.kind());
            } else {
                println!("Other error: {}", e);
            }
        }
    }

    // 使用 anyhow 重新运行，展示错误链打印
    println!("\n--- 场景 6：错误链的两种打印格式 ---");
    let err = inner_operation()
        .context("Middle layer context")
        .context("Outer layer context")
        .unwrap_err();

    println!("Display ({}): {}", "{}", err);
    println!("Debug ({:?}): {:?}", "{:?}", err);
    println!("Alternate Display ({:#}): {:#}", "{:#}", err);

    Ok(())
}

fn divide(a: f64, b: f64) -> AnyhowResult<f64> {
    if b == 0.0 {
        return Err(anyhow!("division by zero: {} / {}", a, b));
    }
    Ok(a / b)
}

fn simulate_io_operation(should_fail: bool) -> AnyhowResult<()> {
    if should_fail {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ))?
    }
    Ok(())
}

fn inner_operation() -> AnyhowResult<()> {
    Err(anyhow!("Root cause: database connection timeout"))
}

// === 模拟 HTTP 处理中的错误映射 ===

#[derive(Debug)]
struct HttpResponse {
    status: u16,
    message: String,
}

fn map_library_error_to_http(err: &LibraryError) -> HttpResponse {
    match err {
        LibraryError::NotFound(_) => HttpResponse {
            status: 404,
            message: "Resource not found".into(),
        },
        LibraryError::Validation { .. } => HttpResponse {
            status: 400,
            message: "Invalid request".into(),
        },
        _ => {
            // 内部错误：泛化响应，详细日志在别处记录
            HttpResponse {
                status: 500,
                message: "Internal server error".into(),
            }
        }
    }
}

fn demo_http_mapping() {
    println!("\n--- 场景 7：HTTP 状态码映射 ---");
    let errors = vec![
        LibraryError::NotFound("user#42".into()),
        LibraryError::Validation {
            field: "email".into(),
            value: "not-an-email".into(),
        },
        LibraryError::Io(std::io::Error::new(
            std::io::ErrorKind::Other,
            "disk full",
        )),
    ];

    for err in errors {
        let resp = map_library_error_to_http(&err);
        println!(
            "LibraryError({}) => HTTP {} {}",
            err, resp.status, resp.message
        );
    }
}

fn main() {
    if let Err(e) = run_app() {
        eprintln!("Application failed: {:#}", e);
    }

    demo_http_mapping();

    println!("\n--- 场景 8：non_exhaustive 保护 ---");
    println!(
        "LibraryError 标记为 #[non_exhaustive]，下游 match 必须包含通配分支 _。\
        这意味着库可以在未来版本安全地添加新错误变体，不会破坏编译。"
    );
}
