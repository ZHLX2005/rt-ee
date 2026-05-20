// Rust 枚举、异常与错误处理方案
//
// 设计意图：
// - 展示 thiserror 如何自动化错误枚举的样板代码
// - 展示 anyhow 如何简化应用层错误处理
// - 展示 panic 与 Result 的边界
// - 展示 ? 运算符和 From trait 的协作

use anyhow::{Context, Result as AnyhowResult};
use thiserror::Error;

// === 库层：用 thiserror 定义精确错误类型 ===

#[derive(Error, Debug)]
enum DatabaseError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Query timeout after {0}ms")]
    Timeout(u64),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

// 模拟数据库操作
struct Database;

impl Database {
    fn connect(conn_str: &str) -> Result<Self, DatabaseError> {
        if conn_str.is_empty() {
            return Err(DatabaseError::Connection("empty connection string".into()));
        }
        println!("Connected to database");
        Ok(Database)
    }

    fn query(&self, sql: &str) -> Result<Vec<String>, DatabaseError> {
        if sql.contains("DROP") {
            return Err(DatabaseError::NotFound("table".into()));
        }
        Ok(vec![
            format!("Result of: {}", sql),
            "Alice".into(),
            "Bob".into(),
        ])
    }
}

// === 应用层：用 anyhow 消费错误 ===

fn run_application() -> AnyhowResult<()> {
    // 应用层不关心具体是 DatabaseError::Connection 还是 DatabaseError::Timeout
    // 只关心"连接数据库时出错了"
    let db = Database::connect("postgres://localhost/mydb")
        .context("Failed to connect to database")?;

    let users = db
        .query("SELECT * FROM users")
        .context("Failed to query users")?;

    println!("Users: {:?}", users);
    Ok(())
}

fn run_application_with_bad_query() -> AnyhowResult<()> {
    let db = Database::connect("postgres://localhost/mydb")
        .context("Failed to connect to database")?;

    // 这个会失败，展示 anyhow 的错误链
    let _users = db
        .query("DROP TABLE users")
        .context("Failed to query users")?;

    Ok(())
}

// === 枚举作为错误：手动 vs thiserror ===

// 手动实现（繁琐）
#[derive(Debug)]
enum ManualError {
    NotFound(u64),
    InvalidInput(String),
}

impl std::fmt::Display for ManualError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManualError::NotFound(id) => write!(f, "Not found: {}", id),
            ManualError::InvalidInput(s) => write!(f, "Invalid: {}", s),
        }
    }
}

impl std::error::Error for ManualError {}

// thiserror 实现（简洁）
#[derive(Error, Debug)]
enum AutoError {
    #[error("Not found: {0}")]
    NotFound(u64),

    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

// === panic 边界 ===

fn risky_operation() {
    panic!("This is a bug, not an error");
}

fn isolated_risky_operation() -> Result<(), Box<dyn std::any::Any + Send>> {
    // catch_unwind 用于隔离 panic，不是常规错误处理
    std::panic::catch_unwind(|| {
        risky_operation();
    })
}

// === 自定义 Result 别名 ===

type DbResult<T> = Result<T, DatabaseError>;

fn find_user_by_id(id: u64) -> DbResult<String> {
    if id == 0 {
        return Err(DatabaseError::NotFound("user #0".into()));
    }
    Ok(format!("User {}", id))
}

fn main() {
    println!("=== 正常流程 ===");
    if let Err(e) = run_application() {
        eprintln!("Error: {}", e);
    }

    println!("\n=== 错误链展示 ===");
    if let Err(e) = run_application_with_bad_query() {
        eprintln!("Error: {}", e);
        // 打印因果链
        if let Some(source) = e.source() {
            eprintln!("Caused by: {}", source);
        }
    }

    println!("\n=== 自定义 Result 别名 ===");
    match find_user_by_id(42) {
        Ok(user) => println!("Found: {}", user),
        Err(e) => eprintln!("DbError: {}", e),
    }

    println!("\n=== panic 隔离 ===");
    match isolated_risky_operation() {
        Ok(_) => println!("Unexpected success"),
        Err(_) => println!("Caught panic from risky_operation"),
    }

    println!("\n=== Option <-> Result 转换 ===");
    let maybe_value: Option<i32> = None;
    let result: Result<i32, &str> = maybe_value.ok_or("value was None");
    println!("Option to Result: {:?}", result);
}
