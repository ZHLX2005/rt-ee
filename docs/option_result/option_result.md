# Option 与 Result

## 设计背景与问题域

### 核心问题：如何表示"可能存在或不存在"的值？

编程中有很多"可能有值，也可能没有"的场景：
- 哈希表中查找一个键
- 数组索引越界
- 数据库查询没有结果

**传统语言的方案**：
- C：返回 NULL/0/-1 表示不存在
- Java：返回 null（NPE 的根源）
- Go：返回 (value, error)

**Rust 的方案**：`Option<T>` + `Result<T, E>`

---

## Option<T>：可空值的类型安全表示

### Option 枚举

```rust
enum Option<T> {
    Some(T),  // 有值
    None,     // 无值
}
```

**为什么不用 null？**

| 维度 | null | Option |
|------|------|--------|
| 类型安全 | 任何引用可以是 null | 编译器强制检查 |
| NPE | 运行时才会发现 | 编译时就强制处理 |
| 组合性 | 难以链式操作 | map, and_then 等方法 |

### Option 的方法

```rust
let x: Option<i32> = Some(5);

// map：变换内部值
let y = x.map(|v| v * 2); // Some(10)

// unwrap_or：默认值
let z = x.unwrap_or(0); // 5

// is_some / is_none：检查
if x.is_some() {
    println!("{}", x.unwrap());
}

// if let：简化匹配
if let Some(v) = x {
    println!("{}", v);
}
```

---

## Result<T, E>：错误的类型安全表示

### Result 枚举

```rust
enum Result<T, E> {
    Ok(T),   // 成功
    Err(E),  // 失败
}
```

**为什么 Result 不是异常？**

| 维度 | 异常 | Result |
|------|------|--------|
| 类型安全 | 运行时才知道 | 编译器强制检查 |
| 性能 | 创建有开销 | 零开销 |
| 可见性 | 隐式传播 | `?` 显式传播 |
| 穷尽性 | 不强制处理 | match 穷尽检查 |

---

## 代码示例（带设计意图注释）

### 示例 1：Option 基本用法

```rust
// 设计意图：Option 让"可能为空"变成类型系统的一部分
// 对比：Java 的 null 没有类型标记

fn find_user_by_id(users: &[(&str, i32)], id: i32) -> Option<&'static str> {
    for (name, user_id) in users {
        if *user_id == id {
            return Some(*name);
        }
    }
    None
}

fn main() {
    let users = vec![("Alice", 1), ("Bob", 2), ("Charlie", 3)];

    match find_user_by_id(&users, 2) {
        Some(name) => println!("Found: {}", name),
        None => println!("User not found"),
    }

    // 使用 if let 更简洁
    if let Some(name) = find_user_by_id(&users, 5) {
        println!("Found: {}", name);
    } else {
        println!("User not found");
    }
}
```

### 示例 2：Option 链式操作

```rust
// 设计意图：Option 的 combinator 方法让嵌套判断更简洁

struct User {
    name: String,
    email: Option<String>,
}

fn main() {
    let user = User {
        name: String::from("Alice"),
        email: Some(String::from("alice@example.com")),
    };

    // 不用 unwrap，避免 panic
    // 使用 map 和 and_then
    let email_domain = user.email
        .as_ref()
        .map(|email| email.split('@').nth(1))
        .flatten();

    println!("Email domain: {:?}", email_domain);

    // 或者用更简洁的写法
    let email_domain2 = user.email
        .as_ref()
        .and_then(|email| email.split('@').nth(1));

    println!("Email domain 2: {:?}", email_domain2);
}
```

### 示例 3：Result 基本用法

```rust
// 设计意图：Result 让错误处理成为类型系统的一部分
// 对比：Java 的 checked exception 需要声明，不强制处理 unchecked

use std::fs::File;
use std::io::{self, Read};

fn read_file_contents(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?; // ? 传播错误
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() {
    match read_file_contents("Cargo.toml") {
        Ok(contents) => println!("Read {} bytes", contents.len()),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

### 示例 4：Result 组合器

```rust
// 设计意图：Result 的组合器让错误处理更流畅

fn parse_port(s: &str) -> Result<u16, std::num::ParseIntError> {
    s.trim().parse::<u16>()
}

fn connect(port_str: &str) -> Result<String, &'static str> {
    let port = parse_port(port_str)
        .map_err(|_| "Invalid port number")?;

    if port > 1024 {
        Ok(format!("Connected to port {}", port))
    } else {
        Err("Port must be greater than 1024")
    }
}

fn main() {
    println!("{:?}", connect("8080"));  // Ok("Connected to port 8080")
    println!("{:?}", connect("abc"));   // Err("Invalid port number")
    println!("{:?}", connect("80"));   // Err("Port must be greater than 1024")
}
```

### 示例 5：Option 与 Result 转换

```rust
// 设计意图：Option 和 Result 可以互相转换

fn find_user(id: u32) -> Option<String> {
    if id == 1 {
        Some(String::from("Alice"))
    } else {
        None
    }
}

fn main() {
    let id = 1;

    // Option -> Result (ok_or)
    let user = find_user(id).ok_or("User not found")?;
    println!("Found: {}", user);

    // Result -> Option (ok)
    let result: Result<u32, &str> = Ok(42);
    println!("Result: {:?}", result.ok()); // Some(42)

    // Option -> Result (ok_or_else)
    let none_opt: Option<i32> = None;
    let result = none_opt.ok_or_else(|| "Default error");
    println!("Result: {:?}", result); // Err("Default error")
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust Option | Java | Go |
|------|------------|------|-----|
| **空值表示** | Option 枚举 | null | nil |
| **类型安全** | 编译器强制处理 | NPE 运行时 | nil 运行时 |
| **链式操作** | map, and_then | Optional (Java 8+) | 无 |
| **穷尽检查** | match 穷尽 | switch 不穷尽 | 无 |

### Java 的 Optional

```java
// Java 8 引入 Optional
Optional<String> name = Optional.of("Alice");

name.map(String::toUpperCase)
    .ifPresent(System.out::println);
```

**问题**：
- Java 的 null 仍然存在，Optional 只是包装
- 不是所有 API 都返回 Optional

### Go 的多返回值

```go
// Go 用 (value, error) 表示可能失败
func findUser(id int) (string, error) {
    if id == 1 {
        return "Alice", nil
    }
    return "", errors.New("not found")
}

name, err := findUser(1)
if err != nil {
    log.Fatal(err)
}
fmt.Println(name)
```

**问题**：
- error 可能被忽略
- 没有类型系统强制

---

## 设计哲学

### Option 是 Maybe Monad

Option 体现了函数式编程的 **Maybe Monad** 概念：

```rust
// 嵌套判断用 flatMap/and_then 扁平化
fn get_city_code(
    person: &Person,
) -> Option<String> {
    person.company
        .as_ref()
        .and_then(|c| c.address.as_ref())
        .and_then(|a| a.city_code.clone())
}
```

**核心思想**：避免嵌套的 if let / match，让逻辑更线性。

### Result 是 Either Monad

Result 体现了 **Either Monad**：

```rust
// 错误传播用 ? 运算符
fn read_and_parse(path: &str) -> Result<i32, ParseError> {
    let content = read_file(path)?; // 错误直接返回
    parse(content)                  // 成功继续
}
```

**核心思想**：错误作为值的一部分，沿着调用链传播。

---

## 总结

| 概念 | 说明 |
|------|------|
| Option<T> | 有值（Some）或无值（None） |
| Result<T, E> | 成功（Ok）或失败（Err） |
| ? 运算符 | 错误传播的语法糖 |
| map/and_then | Option/Result 的组合器 |

**核心洞察**：Rust 的 Option 和 Result 将"可能为空"或"可能失败"变成类型系统的一部分，编译器强制程序员处理所有情况，从根本上消灭了 NPE 和被忽略的错误。
