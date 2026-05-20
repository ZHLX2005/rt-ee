# 错误处理 (Error Handling)

## 设计背景与问题域

### 核心问题：如何处理可能失败的操作？

编程中充满了各种可能的失败：
- 文件不存在
- 网络连接失败
- JSON 解析错误
- 除零错误

**传统语言的方案**：
- C：返回错误码（-1, NULL）
- Java：抛出异常
- Go：返回 (value, error)

**Rust 的方案**：`Result<T, E>` 类型 + `?` 运算符

### 为什么 Rust 不用异常？

| 维度 | 异常 | Result |
|------|------|--------|
| 控制流 | 跳转（try-catch） | 正常返回值 |
| 类型安全 | 泛型异常（Java）or any（Go） | 编译器强制检查 |
| 性能 | 创建异常有开销 | 零开销 |
| 可见性 | 可能被忽略（Java 的 throws 可省略） | 必须处理或传递 |
| 并发 | 异常在线程间传播 | Result 沿着调用链传递 |

**Rust 的设计选择**：异常会破坏 Rust 的"确定性析构"保证。如果在栈展开（unwinding）过程中抛出异常，Drop 可能被跳过。

---

## 抽象设计分析

### Result 类型：可恢复错误的载体

```rust
enum Result<T, E> {
    Ok(T),   // 成功，持有成功值
    Err(E),  // 失败，持有错误值
}
```

**为什么 Result 是枚举而不是异常？**

1. **类型系统强制**：函数返回 Result，调用者必须处理
2. **组合性**：Result 有丰富的 combinator 方法（map, and_then 等）
3. **明确性**：`?` 运算符让错误传播清晰可见

### Option 类型：可空值的载体

```rust
enum Option<T> {
    Some(T),  // 有值
    None,     // 无值
}
```

**Option vs Result**：

| 类型 | 用途 | 成功时的值 |
|------|------|-----------|
| Option | 值可能不存在 | Some(T) |
| Result | 操作可能失败 | Ok(T) |

### 错误类型的设计

```rust
use std::fmt;

// 自定义错误类型
#[derive(Debug)]
enum MyError {
    Io(std::io::Error),
    Parse(std::num::ParseIntError),
}

impl fmt::Display for MyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MyError::Io(e) => write!(f, "IO error: {}", e),
            MyError::Parse(e) => write!(f, "Parse error: {}", e),
        }
    }
}
```

---

## 核心规则

### ? 运算符：错误传播的语法糖

```rust
// 不用 ?
fn read_first_line(path: &str) -> Result<String, std::io::Error> {
    let file = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return Err(e),
    };
    let mut reader = std::io::BufReader::new(file);
    let mut line = String::new();
    match reader.read_line(&mut line) {
        Ok(_) => Ok(line),
        Err(e) => Err(e),
    }
}

// 用 ?
fn read_first_line(path: &str) -> Result<String, std::io::Error> {
    let file = std::fs::File::open(path)?; // 错误直接返回
    let mut reader = std::io::BufReader::new(file);
    let mut line = String::new();
    reader.read_line(&mut line)?; // 错误直接返回
    Ok(line)
}
```

### From trait：自动错误转换

```rust
use std::num::ParseIntError;

fn parse_and_add(a: &str, b: &str) -> Result<i32, ParseIntError> {
    let a: i32 = a.parse()?; // &str -> i32
    let b: i32 = b.parse()?; // &str -> i32
    Ok(a + b)
}
```

### 错误链

```rust
use std::io;
use std::num::ParseIntError;

fn read_and_parse(path: &str) -> Result<i32, Box<dyn std::error::Error>> {
    // Box<dyn Error> 可以容纳任何实现了 Error trait 的错误
    let content = std::fs::read_to_string(path)?; // io::Error
    let number: i32 = content.trim().parse()?;       // ParseIntError
    Ok(number)
}
```

---

## 代码示例（带设计意图注释）

### 示例 1：基本错误处理

```rust
// 设计意图：Result 强制调用者处理可能的错误
// 对比：Java 的 checked exception 需要 throws 声明，unchecked 不强制

use std::fs::File;
use std::io::{self, Read};

fn read_file_contents(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?; // ? 运算符：失败直接返回
    let mut contents = String::new();
    file.read_to_string(&mut contents)?; // 失败直接返回
    Ok(contents)
}

fn main() {
    match read_file_contents("Cargo.toml") {
        Ok(contents) => println!("File contents: {}", contents),
        Err(e) => eprintln!("Error reading file: {}", e),
    }
}
```

### 示例 2：使用闭包组合器

```rust
// 设计意图：Result 的 combinator 方法让错误处理更具表达力

fn parse_and_double(s: &str) -> Result<i32, std::num::ParseIntError> {
    s.trim()
        .parse::<i32>()       // Result<i32, ParseIntError>
        .map(|n| n * 2)       // 如果 OK，乘以 2
        .map_err(|e| {
            eprintln!("Parse error: {}", e);
            e
        })
}

fn main() {
    println!("{:?}", parse_and_double("42"));   // Ok(84)
    println!("{:?}", parse_and_double("oops"));  // Err(...)
}
```

### 示例 3：Option 与 Result 转换

```rust
// 设计意图：Option 和 Result 可以互相转换

fn find_user_by_id(id: u32) -> Option<String> {
    if id == 1 {
        Some(String::from("Alice"))
    } else {
        None
    }
}

fn find_user_by_name(name: &str) -> Result<u32, &'static str> {
    if name == "Alice" {
        Ok(1)
    } else {
        Err("User not found")
    }
}

fn main() {
    // Option -> Result
    let user = find_user_by_id(1).ok_or("Not found")?;
    println!("Found: {}", user);

    // Result -> Option
    let id = find_user_by_name("Bob").ok();
    println!("ID: {:?}", id);
}
```

### 示例 4：自定义错误类型

```rust
// 设计意图：自定义错误类型让错误信息更精确

use std::fmt;

#[derive(Debug)]
enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    NotFound(String),
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => write!(f, "Connection failed: {}", msg),
            DatabaseError::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
            DatabaseError::NotFound(id) => write!(f, "Record not found: {}", id),
        }
    }
}

fn get_user(id: u32) -> Result<String, DatabaseError> {
    if id == 0 {
        Err(DatabaseError::NotFound(format!("User {} not found", id)))
    } else {
        Ok(format!("User #{}", id))
    }
}
```

### 示例 5：anyhow：简化错误处理

```rust
// 设计意图：anyhow 库简化了动态错误类型的处理
// 适合应用层代码，不需要精确的错误类型

use anyhow::{Context, Result};

fn read_config(path: &str) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config from {}", path))?;
    Ok(content)
}

// 调用方不需要知道具体错误类型
fn main() -> Result<()> {
    let config = read_config("config.toml")?;
    println!("Config: {}", config);
    Ok(())
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **错误表示** | Result 枚举 | Exception（checked/unchecked） | 多返回值 (value, error) |
| **强制检查** | 编译器强制 | Checked 异常强制，Unchecked 不强制 | 不强制 |
| **传播语法** | `?` 运算符 | `throws` 声明 | 直接返回 |
| **错误链** | Box<dyn Error> | cause | 包装 error |
| **性能** | 零开销 | 异常创建有开销 | 零开销 |

### Java 的异常机制

```java
// Checked Exception：必须声明或捕获
public String readFile(String path) throws IOException {
    BufferedReader reader = new BufferedReader(new FileReader(path));
    return reader.readLine();
}

// 调用者必须处理
try {
    String line = readFile("hello.txt");
} catch (IOException e) {
    e.printStackTrace();
}
```

**问题**：
- Checked 异常会污染函数签名
- Unchecked 异常可以忽略
- 异常创建有性能开销

### Go 的错误处理

```go
func readFile(path string) (string, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return "", err // 手动传播
    }
    return string(data), nil
}

// 调用者检查
data, err := readFile("hello.txt")
if err != nil {
    log.Fatal(err)
}
fmt.Println(string(data))
```

**问题**：
- 错误可能被忽略（不检查 err）
- 大量 `if err != nil` 样板代码
- 没有类型系统强制

---

## 设计哲学

### 为什么 Rust 选择 Result 而不是异常？

1. **显式优于隐式**：`?` 让错误传播清晰可见，不会有隐藏的跳转
2. **性能**：Result 是普通枚举，创建没有异常的开销
3. **组合性**：map, and_then 等 combinator 让错误处理富有表达力
4. **确定性**：错误处理不会触发栈展开，保证 Drop 被调用

### 错误处理的分层

```
应用层（anyhow）
    ↓
业务逻辑层（thiserror）
    ↓
基础设施层（具体错误类型）
```

**anyhow**：适合应用层，不需要精确的错误类型
**thiserror**：适合库层，定义结构化的错误类型

---

## 总结

| 概念 | 说明 |
|------|------|
| `Result<T, E>` | 表示操作成功（Ok）或失败（Err） |
| `Option<T>` | 表示值存在（Some）或不存在（None） |
| `?` 运算符 | 错误传播的语法糖 |
| `map` / `and_then` | Result 的组合器方法 |
| anyhow / thiserror | 简化错误处理的库 |

**核心洞察**：Rust 的错误处理是"显式优于隐式"设计哲学的体现——错误必须被处理或显式传播，而不是被隐藏或忽略。
