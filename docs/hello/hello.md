# Hello World

## 设计背景与问题域

### 核心问题：Rust 程序是如何运行的？

每个语言入门都是从 Hello World 开始，但 Rust 的 Hello World 揭示了几个关键概念：

```rust
fn main() {
    println!("Hello, world!");
}
```

### 这个程序背后的关键概念

1. **`fn main()`**：程序入口点
2. **`println!` 宏**：格式化输出
3. **编译**：`rustc main.rs`
4. **执行**：生成的可执行文件

---

## 抽象设计分析

### fn main() 的特殊性

```rust
// main 函数没有参数，没有返回值
// 它是程序的入口点，由运行时调用

fn main() {
    // 程序从这里开始执行
}
```

**为什么是 `main`？**
- C/C++ 的约定
- 操作系统加载可执行文件后，调用 `main` 函数
- Rust 保持这个约定以便与 C ABI 互操作

### println! 宏

```rust
// println! 是宏，不是函数
// 宏在编译时展开，可以接受可变参数

println!("Hello, {}!", "world");        // Hello, world!
println!("Value: {}", 42);               // Value: 42
println!("{:?}", vec![1, 2, 3]);        // [1, 2, 3]
```

**为什么用宏？**
- 函数无法直接接受可变数量的泛型参数
- 编译时格式化，避免运行时开销

---

## 代码示例（带设计意图注释）

### 示例 1：Rust 程序结构

```rust
// 设计意图：展示 Rust 程序的基本结构
// 对比：Java 需要 class 包装，Go 更简洁

// Rust：直接入口点
fn main() {
    println!("Hello, Rust!");
}
```

### 示例 2：多个文件项目

### src/main.rs
```rust
mod greeting;

fn main() {
    greeting::hello();
}
```

### src/greeting.rs
```rust
pub fn hello() {
    println!("Hello from another file!");
}
```

### 示例 3：Cargo 项目

### Cargo.toml
```toml
[package]
name = "hello-project"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "hello"
path = "src/main.rs"
```

### src/main.rs
```rust
fn main() {
    println!("Hello from Cargo project!");
}
```

### 示例 4：格式化输出

```rust
// 设计意图：展示 println! 的格式化能力
// 对比：Java 的 System.out.printf，Go 的 fmt.Printf

fn main() {
    // 基本字符串
    println!("Hello, world!");

    // 占位符
    let name = "Alice";
    let age = 30;
    println!("Name: {}, Age: {}", name, age);

    // 调试格式化
    let numbers = vec![1, 2, 3];
    println!("Debug: {:?}", numbers);

    // 对齐和填充
    println!("{:>10}", "right");    // "     right"
    println!("{:<10}", "left");     // "left      "
    println!("{:^10}", "center");   // "  center  "

    // 数字格式化
    println!("{:?}", 255);          // 255
    println!("{:#x}", 255);          // 0xff
    println!("{:#b}", 255);          // 0b11111111
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **入口点** | fn main() | 必须有 class | 必须有 package main |
| **文件结构** | 任意 | 必须与类名对应 | 必须与文件名对应 |
| **编译** | rustc / cargo | javac | go build |
| **输出** | println! | System.out.println | fmt.Println |

### Java 的 Hello World

```java
public class Hello {
    public static void main(String[] args) {
        System.out.println("Hello, world!");
    }
}
```

**问题**：
- 必须有 class 包装
- 必须 public class 与文件名相同

### Go 的 Hello World

```go
package main

import "fmt"

func main() {
    fmt.Println("Hello, world!")
}
```

**问题**：
- 必须 package main
- 必须有 import

---

## Rust 程序的编译与运行

### rustc 直接编译

```bash
rustc main.rs      # 生成 main.exe
./main.exe         # Windows
./main             # Linux/macOS
```

### Cargo 项目管理

```bash
cargo new hello-project  # 创建新项目
cargo build              # 编译
cargo run                # 运行
cargo build --release    # Release 优化
```

### 编译过程

```
源代码 (.rs)
    ↓
Lexer → Parser → AST
    ↓
类型检查 + 借用检查
    ↓
LLVM IR 生成
    ↓
机器码生成
    ↓
可执行文件
```

---

## 设计哲学

### 简洁但有表达力

```rust
fn main() {
    // 一行代码，零配置
    println!("Hello, world!");
}
```

**Rust 的设计目标**：
- 入门简单
- 但能表达复杂系统
- 无运行时开销

### zero-cost abstraction

```rust
println!("{}", 42);
```

**这个 println! 调用**：
- 编译时格式化
- 没有反射开销
- 接近直接系统调用的性能

---

## 总结

| 概念 | 说明 |
|------|------|
| fn main() | 程序入口点 |
| println! | 格式化输出宏 |
| rustc | Rust 编译器 |
| cargo | 构建工具和包管理器 |

**核心洞察**：Rust 的 Hello World 展示了 Rust 的设计哲学——简洁的语法，但包含编译时检查、零成本抽象等深层概念。
