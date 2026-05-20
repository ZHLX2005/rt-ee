# 结构体 (Structs)

## 设计背景与问题域

### 核心问题：如何组合相关数据？

现实世界中的实体通常有多个属性：
- 人：有名字、年龄、地址
- 矩形：有宽度和高度
- 服务器：有 IP、端口、状态

**传统语言的方案**：
- C：struct（只有数据）
- Java：class（数据 + 方法）
- Go：struct（数据 + 方法，但更简单）

**Rust 的方案**：struct + impl 分离数据和方法

---

## 抽象设计分析

### Rust Struct vs Java Class

| 维度 | Rust Struct | Java Class |
|------|-------------|-----------|
| **数据** | 字段 | 字段 |
| **方法** | 单独的 impl 块 | 直接在类里 |
| **继承** | 不支持（用 Trait 组合） | 支持 |
| **默认字段值** | 必须显式初始化 | 有默认值 |
| **构造函数** | 命名函数模式 | 构造函数 |

### Rust Struct vs Go Struct

| 维度 | Rust | Go |
|------|------|-----|
| **方法** | impl 块 | receiver 参数 |
| **可见性** | pub 字段可选 | 字段公开 |
| **泛型** | 支持 | 有限支持 |

---

## 核心规则

### 三种 Struct

```rust
// 1. 命名字段结构体
struct User {
    name: String,
    email: String,
    active: bool,
}

// 2. 元组结构体（用于区分类型）
struct Color(u8, u8, u8);

// 3. 单元结构体（用于 trait 实现）
struct Marker;
```

### impl 方法

```rust
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    // 关联函数（不需要 self）
    fn square(size: u32) -> Rectangle {
        Rectangle { width: size, height: size }
    }

    // 实例方法（需要 &self）
    fn area(&self) -> u32 {
        self.width * self.height
    }

    // 可变方法
    fn set_width(&mut self, width: u32) {
        self.width = width;
    }
}
```

---

## 代码示例（带设计意图注释）

### 示例 1：基本结构体

```rust
// 设计意图：结构体将相关数据组合在一起
// 对比：Java 的类更重，Rust 的 struct 更轻量

struct User {
    name: String,
    email: String,
    active: bool,
}

impl User {
    // 关联函数：创建 User 的工厂方法
    fn new(name: &str, email: &str) -> User {
        User {
            name: String::from(name),
            email: String::from(email),
            active: true,
        }
    }

    // 实例方法
    fn greet(&self) {
        println!("Hello, {}!", self.name);
    }
}

fn main() {
    let user = User::new("Alice", "alice@example.com");
    user.greet();
}
```

### 示例 2：元组结构体

```rust
// 设计意图：元组结构体用于区分类型，但字段没有命名
// 典型应用：RGB 颜色、2D 坐标、HTTP 状态码

struct Color(u8, u8, u8); // RGB
struct Point(u32, u32);   // 2D 坐标

impl Color {
    fn red() -> Self {
        Color(255, 0, 0)
    }

    fn brightness(&self) -> u8 {
        (self.0 as u16 + self.1 as u16 + self.2 as u16) as u8 / 3
    }
}

fn main() {
    let red = Color::red();
    let bg = Color(128, 128, 128);

    println!("Red brightness: {}", red.brightness());
    println!("Gray brightness: {}", bg.brightness());
}
```

### 示例 3：结构体与借用

```rust
// 设计意图：结构体的方法通常需要借用 self
// 这样调用者仍然持有结构体的所有权

struct Counter {
    count: i32,
}

impl Counter {
    fn new() -> Counter {
        Counter { count: 0 }
    }

    // &self：不可变借用
    fn get(&self) -> i32 {
        self.count
    }

    // &mut self：可变借用
    fn increment(&mut self) {
        self.count += 1;
    }

    // self：获取所有权（不常用）
    fn consume(self) -> i32 {
        self.count
    }
}

fn main() {
    let mut counter = Counter::new();
    counter.increment();
    counter.increment();
    println!("{}", counter.get()); // 2

    // consume 获取所有权
    let final_count = counter.consume();
    println!("Final: {}", final_count);
    // println!("{}", counter.get()); // 错误！counter 已被消费
}
```

### 示例 4：结构体与 Trait

```rust
// 设计意图：结构体实现 Trait 来获得行为
// 对比：Java 的接口，Go 的方法集合

struct Circle {
    radius: f64,
}

struct Rectangle {
    width: f64,
    height: f64,
}

trait Shape {
    fn area(&self) -> f64;
    fn name(&self) -> &str;
}

impl Shape for Circle {
    fn area(&self) -> f64 {
        std::f64::consts::PI * self.radius * self.radius
    }

    fn name(&self) -> &str {
        "Circle"
    }
}

impl Shape for Rectangle {
    fn area(&self) -> f64 {
        self.width * self.height
    }

    fn name(&self) -> &str {
        "Rectangle"
    }
}

fn print_area(shape: &dyn Shape) {
    println!("{} area: {:.2}", shape.name(), shape.area());
}

fn main() {
    let circle = Circle { radius: 5.0 };
    let rect = Rectangle { width: 4.0, height: 3.0 };

    print_area(&circle); // Circle area: 78.54
    print_area(&rect);  // Rectangle area: 12.00
}
```

### 示例 5：结构体内存布局

```rust
// 设计意图：理解结构体的内存布局有助于性能优化

struct Point3D {
    x: f64,
    y: f64,
    z: f64,
}

fn main() {
    let p = Point3D {
        x: 1.0,
        y: 2.0,
        z: 3.0,
    };

    // 结构体在栈上分配
    println!("Size of Point3D: {} bytes", std::mem::size_of::<Point3D>());

    // 字段内存布局
    println!("x: {:p}, y: {:p}, z: {:p}", &p.x, &p.y, &p.z);
}
```

---

## 与 Java/Go 的深度对比

### Java 的类

```java
public class User {
    private String name;
    private String email;
    private boolean active;

    public User(String name, String email) {
        this.name = name;
        this.email = email;
        this.active = true;
    }

    public void greet() {
        System.out.println("Hello, " + name + "!");
    }
}
```

**关键区别**：
- Java 构造函数与类名相同
- 方法可以直接访问字段
- 支持继承和多态

### Go 的结构体

```go
type User struct {
    Name  string
    Email string
    Active bool
}

func (u *User) Greet() {
    fmt.Printf("Hello, %s!\n", u.Name)
}
```

**关键区别**：
- Go 用 receiver 作为第一个参数
- 没有继承，用组合代替
- 字段可以是未导出的（小写字母开头）

---

## 设计哲学

### 数据与行为分离

Rust 的 struct + impl 分离体现了：
- **数据**在 struct 中定义
- **行为**在 impl 中定义

这比 Java 的"一切皆类"更清晰：
- 结构体关注"是什么"
- Trait 实现关注"能做什么"

### 零成本抽象

```rust
struct Point {
    x: f64,
    y: f64,
}

impl Point {
    fn distance_from_origin(&self) -> f64 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
}
```

**编译器优化**：
- 方法调用可能被内联
- 结构体内存布局由编译器优化
- 没有 Java 那样的虚表开销（除非用 dyn Trait）

---

## 常见模式

### 构造函数模式

```rust
struct Config {
    host: String,
    port: u16,
}

impl Config {
    fn new(host: String, port: u16) -> Self {
        Config { host, port }
    }

    // 默认值模式
    fn default() -> Self {
        Config {
            host: String::from("localhost"),
            port: 8080,
        }
    }
}
```

### Builder 模式

```rust
struct Server {
    host: String,
    port: u16,
    threads: usize,
}

struct ServerBuilder {
    host: String,
    port: u16,
    threads: usize,
}

impl ServerBuilder {
    fn new() -> Self {
        ServerBuilder {
            host: String::from("localhost"),
            port: 8080,
            threads: 4,
        }
    }

    fn host(&mut self, host: String) -> &mut Self {
        self.host = host;
        self
    }

    fn port(&mut self, port: u16) -> &mut Self {
        self.port = port;
        self
    }

    fn threads(&mut self, threads: usize) -> &mut Self {
        self.threads = threads;
        self
    }

    fn build(&self) -> Server {
        Server {
            host: self.host.clone(),
            port: self.port,
            threads: self.threads,
        }
    }
}
```

---

## 总结

| 概念 | 说明 |
|------|------|
| 命名字段结构体 | 字段有名字 |
| 元组结构体 | 字段无名字，用位置访问 |
| 单元结构体 | 无字段，用于 trait 实现 |
| impl | 定义与结构体关联的方法 |

**核心洞察**：Rust 的 struct 是轻量级的数据结构 + 通过 impl 定义行为，相比 Java 的"一切皆类"更灵活，配合 Trait 实现组合优于继承。
