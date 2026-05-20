# 枚举 (Enums)

## 设计背景与问题域

### 核心问题：如何表示"一种类型，多种可能"？

编程中经常需要表示"几种可能的值"：
- 形状：圆形、矩形、三角形
- 消息：退出、移动、写入
- 状态：成功、失败、进行中

**传统语言的方案**：
- C：枚举常量 + 联合（union）
- Java：类层次结构（继承）
- Go：没有 enum，用常量替代

**Rust 的方案**：枚举类型 + 模式匹配

---

## 抽象设计分析

### Rust Enum vs Java/C++ Enum

| 维度 | Rust Enum | Java Enum | C Enum |
|------|-----------|-----------|--------|
| **类型安全** | 枚举变体可以携带数据 | 只能有固定字段 | 只是整数常量 |
| **方法** | 可以 impl 方法 | 可以有方法 | 不能有方法 |
| **数据** | 变体可以携带任意数据 | 可以有字段（Java 14+） | 不能有数据 |
| **匹配** | match 必须穷尽所有情况 | switch 不是穷尽 | switch 可以不穷尽 |

### 枚举是代数数据类型

```rust
enum Message {
    Quit,                           // 无数据变体
    Move { x: i32, y: i32 },      // 命名结构体变体
    Write(String),                  // 元组变体
    ChangeColor(i32, i32, i32),    // 元组变体
}
```

**设计意图**：枚举的每个变体可以有不同的数据结构，这是**标签联合（Tagged Union）**的实现，编译器确保你处理每种情况。

---

## 核心规则

### match 必须穷尽

```rust
enum Color {
    Red,
    Green,
    Blue,
}

fn print_color(c: Color) {
    match c {
        Color::Red => println!("Red"),
        Color::Green => println!("Green"),
        Color::Blue => println!("Blue"),
        // 如果缺少任何分支，编译错误！
    }
}
```

### if let：简化单分支匹配

```rust
let msg = Message::Write(String::from("hello"));

// 单分支匹配用 if let 更简洁
if let Message::Write(text) = msg {
    println!("{}", text);
}
```

### Option<T>：枚举的典型应用

```rust
enum Option<T> {
    Some(T),   // 有值
    None,       // 无值
}

fn find_user(id: u32) -> Option<String> {
    if id == 1 {
        Some(String::from("Alice"))
    } else {
        None
    }
}
```

---

## 代码示例（带设计意图注释）

### 示例 1：消息枚举

```rust
// 设计意图：枚举变体可以携带不同类型的数据
// 对比：如果用结构体，需要额外的 type 字段

enum Message {
    Quit,                           // 无数据的变体
    Move { x: i32, y: i32 },      // 命名结构体变体
    Write(String),                  // 元组变体
    ChangeColor(u8, u8, u8),       // 元组变体
}

impl Message {
    fn call(&self) {
        match self {
            Message::Quit => println!("Quit"),
            Message::Move { x, y } => println!("Move to ({}, {})", x, y),
            Message::Write(text) => println!("Write: {}", text),
            Message::ChangeColor(r, g, b) => println!("Color: {}, {}, {}", r, g, b),
        }
    }
}

fn main() {
    let msgs = vec![
        Message::Quit,
        Message::Move { x: 10, y: 20 },
        Message::Write(String::from("hello")),
        Message::ChangeColor(255, 0, 0),
    ];

    for msg in msgs {
        msg.call();
    }
}
```

### 示例 2：IP地址枚举

```rust
// 设计意图：枚举可以区分不同的变体类型
// 这是 Rust 类型安全的典型应用

enum IpAddrKind {
    V4,
    V6,
}

struct IpAddr {
    kind: IpAddrKind,
    address: String,
}

// 更简洁的方式：枚举直接携带数据
enum IpAddr2 {
    V4(String),
    V6(String),
}

// 带多种数据结构的枚举
enum IpAddr3 {
    V4(u8, u8, u8, u8),           // 点分十进制
    V6(String),                     // IPv6 地址字符串
}

fn main() {
    let home = IpAddr2::V4(String::from("127.0.0.1"));
    let loopback = IpAddr2::V6(String::from("::1"));

    match home {
        IpAddr2::V4(addr) => println!("IPv4: {}", addr),
        IpAddr2::V6(addr) => println!("IPv6: {}", addr),
    }
}
```

### 示例 3：Result 枚举

```rust
// 设计意图：Result 是标准库定义的枚举
// 这展示了枚举在实际应用中的强大威力

enum Result<T, E> {
    Ok(T),
    Err(E),
}

// 实际使用
fn divide(a: f64, b: f64) -> Result<f64, &'static str> {
    if b == 0.0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

fn main() {
    match divide(10.0, 2.0) {
        Ok(result) => println!("Result: {}", result),
        Err(e) => println!("Error: {}", e),
    }
}
```

### 示例 4：状态机枚举

```rust
// 设计意图：枚举可以表示状态机的状态转换
// 每个状态变体可以携带不同的上下文数据

enum OrderState {
    Pending,
    Processing { worker_id: u32 },
    Shipped { tracking_number: String },
    Delivered,
    Cancelled { reason: String },
}

fn process_order(state: OrderState) -> OrderState {
    match state {
        OrderState::Pending => {
            println!("Starting processing...");
            OrderState::Processing { worker_id: 42 }
        }
        OrderState::Processing { worker_id } => {
            println!("Worker {} processing order...", worker_id);
            OrderState::Shipped {
                tracking_number: String::from("TRACK123"),
            }
        }
        OrderState::Shipped { tracking_number } => {
            println!("Order shipped with tracking: {}", tracking_number);
            OrderState::Delivered
        }
        OrderState::Delivered => {
            println!("Order already delivered");
            OrderState::Delivered
        }
        OrderState::Cancelled { reason } => {
            println!("Order already cancelled: {}", reason);
            OrderState::Cancelled { reason }
        }
    }
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust Enum | Java Enum | Go |
|------|-----------|-----------|-----|
| **数据携带** | 每个变体可不同 | 每个枚举值字段相同（Java 14+ 支持） | 无 enum，用常量 |
| **方法** | 可以 impl | 可以有方法 | 无 |
| **模式匹配** | match 穷尽检查 | switch 不穷尽 | 无 |
| **类型安全** | 变体类型不同 | 枚举值类型相同 | 只是整数常量 |

### Java 的 Enum

```java
public enum Color {
    RED(255, 0, 0),
    GREEN(0, 255, 0),
    BLUE(0, 0, 255);

    private final int r, g, b;

    Color(int r, int g, int b) {
        this.r = r;
        this.g = g;
        this.b = b;
    }

    public int getR() { return r; }
}
```

**关键区别**：Java 的枚举值必须结构相同，Rust 的枚举变体可以结构不同。

### Rust 枚举的独特优势

1. **穷尽匹配**：编译器确保你处理所有情况
2. **变体类型差异**：不同的变体可以携带不同类型的数据
3. **方法实现**：枚举可以有 impl 块
4. **标准库应用**：Option, Result 都是枚举

---

## 设计哲学

### 枚举是"和类型"（Sum Type）

**代数数据类型**：
- **枚举（和类型）**：A | B | C —— 值可以是 A 或 B 或 C
- **结构体（积类型）**：A & B —— 值同时是 A 和 B

```rust
// 和类型：值可以是其中之一
enum Color {
    Red,
    Green,
    Blue,
}

// 积类型：值同时具有所有字段
struct RGB {
    r: u8,
    g: u8,
    b: u8,
}
```

**为什么这重要？**
- 编译器在编译时就排除"无效状态"
- match 的穷尽检查确保没有遗漏
- 类型系统保证状态转换的合法性

---

## 总结

| 概念 | 说明 |
|------|------|
| 枚举变体 | 枚举的值可以是不同变体之一 |
| match | 必须穷尽所有变体，否则编译错误 |
| if let | 单分支匹配的简化语法 |
| Option/Result | 标准库定义的枚举，展示枚举的强大 |

**核心洞察**：Rust 的枚举是代数数据类型，通过标签联合保证了类型安全——每种可能的值都是一种明确定义的变体，编译器确保你处理所有情况。
