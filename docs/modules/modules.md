# 模块 (Modules)

## 设计背景与问题域

### 核心问题：如何组织代码，控制可见性？

随着代码增长，需要：
- 将代码分割成逻辑单元
- 控制哪些代码对外可见
- 避免命名冲突

**传统语言的方案**：
- C：文件和 static 函数
- Java：package + class
- Go：package（单一文件）

**Rust 的方案**：mod + pub + use

---

## 抽象设计分析

### 模块系统核心概念

```rust
mod outer {
    pub fn public_function() { }      // 对外可见
    fn private_function() { }        // 仅在 outer 内可见

    pub mod inner {
        pub fn inner_function() { }  // outer::inner::inner_function
    }
}

fn main() {
    outer::public_function();
    outer::inner::inner_function();
}
```

### pub 的可见性层级

| 修饰符 | 可见性 |
|--------|--------|
| (无) | 仅当前模块 |
| pub | 所有模块 |
| pub(crate) | 仅当前 crate |
| pub(super) | 父模块 |
| pub(in path) | 指定路径的模块 |

---

## 代码示例（带设计意图注释）

### 示例 1：基本模块

```rust
// 设计意图：模块将相关功能组织在一起
// 对比：类似于 Java 的 package，但更简洁

mod authentication {
    pub fn login(username: &str, password: &str) -> Result<(), AuthError> {
        if username == "admin" && password == "secret" {
            Ok(())
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }

    fn hash_password(password: &str) -> String {
        // 内部实现，外部不可访问
        format!("hashed_{}", password)
    }
}

#[derive(Debug)]
enum AuthError {
    InvalidCredentials,
}

fn main() {
    match authentication::login("admin", "secret") {
        Ok(_) => println!("Login successful"),
        Err(e) => println!("Login failed: {:?}", e),
    }
}
```

### 示例 2：嵌套模块与路径

```rust
// 设计意图：模块可以嵌套，形成命名空间层次

mod network {
    pub mod http {
        pub fn get(url: &str) -> String {
            format!("GET {}", url)
        }

        pub mod headers {
            pub fn content_type() -> &'static str {
                "application/json"
            }
        }
    }

    pub mod websocket {
        pub fn connect(url: &str) {
            println!("Connecting to {}", url);
        }
    }
}

fn main() {
    // 使用完整路径
    println!("{}", network::http::get("https://example.com"));
    println!("{}", network::http::headers::content_type());

    // 使用 use 简化
    use network::websocket;
    websocket::connect("wss://example.com");
}
```

### 示例 3：use 与重导出

```rust
// 设计意图：use 导入路径，pub use 重导出

mod shapes {
    pub mod circle {
        pub fn area(radius: f64) -> f64 {
            std::f64::consts::PI * radius * radius
        }
    }

    pub mod rectangle {
        pub fn area(width: f64, height: f64) -> f64 {
            width * height
        }
    }

    // 重导出：让外部可以直接访问
    pub use circle::area as circle_area;
    pub use rectangle::area as rectangle_area;
}

fn main() {
    // 直接使用重导出的名称
    println!("{}", shapes::circle_area(5.0));
    println!("{}", shapes::rectangle_area(3.0, 4.0));
}
```

### 示例 4：模块与所有权

```rust
// 设计意图：模块不影响所有权规则

mod counter {
    pub struct Counter {
        count: i32, // 私有字段
    }

    impl Counter {
        pub fn new() -> Counter {
            Counter { count: 0 }
        }

        pub fn increment(&mut self) {
            self.count += 1;
        }

        // 访问私有字段的方法
        pub fn get(&self) -> i32 {
            self.count
        }
    }
}

fn main() {
    let mut counter = counter::Counter::new();
    counter.increment();
    counter.increment();
    println!("{}", counter.get()); // 2

    // 无法直接访问私有字段
    // counter.count = 10; // 编译错误！
}
```

### 示例 5：crate 与 mod.rs

### 文件结构

```
src/
├── main.rs
├── lib.rs
└── network/
    ├── mod.rs      // 方式 1：mod.rs
    ├── http.rs     // 方式 2：直接文件
    └── websocket.rs
```

### main.rs
```rust
mod network; // 声明模块

fn main() {
    network::http::get("https://example.com");
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **模块定义** | mod 关键字 | package | package |
| **可见性** | pub 修饰符 | public/private | 首字母大写 |
| **导入** | use | import | import |
| **文件组织** | 灵活 | 必须与 package 名对应 | 必须与文件名对应 |
| **可见性层级** | pub, pub(crate) 等 | 4 级 | 只有包级 |

### Java 的 package

```java
package com.example.network;

public class HttpClient {
    public void get(String url) { }
}
```

```java
import com.example.network.HttpClient;
```

**问题**：
- package 必须与目录结构严格对应
- 没有模块层级的可见性控制

### Go 的 package

```go
package network

func Get(url string) string {
    return "GET " + url
}
```

```go
import "example.com/project/network"
```

**问题**：
- 所有导出由首字母大写控制
- 没有私有模块的概念

---

## 设计哲学

### 可见性是编译时保证

```rust
mod inner {
    fn private_function() { }

    pub fn public_function() {
        private_function(); // OK，内部可以访问
    }
}

fn main() {
    // inner::private_function(); // 编译错误！
}
```

**核心思想**：Rust 的可见性由编译器强制，运行时没有开销。

### 路径与导入

```rust
// 绝对路径：从 crate 开始
crate::module::function()

// 相对路径：从当前模块开始
super::parent_function()
self::sibling_function()
```

---

## 总结

| 概念 | 说明 |
|------|------|
| mod | 声明模块 |
| pub | 公开可见性 |
| use | 导入路径 |
| pub use | 重导出 |
| crate | 当前 crate 的根 |

**核心洞察**：Rust 的模块系统提供了精确的可见性控制（pub, pub(crate) 等），比 Java 和 Go 更灵活，同时保持编译时检查。
