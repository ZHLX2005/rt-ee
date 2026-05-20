# Rust 特有的设计范式与语言级抽象

## 设计背景与问题域

Java 和 Go 程序员已经熟悉了许多经典设计模式：工厂模式、观察者模式、策略模式等。但 Rust 的设计范式**不是**在 OOP 框架内添加新招式，而是从**类型系统**和**编译期保证**的角度重新定义了"安全"和"抽象"的含义。

Rust 的核心设计问题不是"如何组织对象"，而是：

1. **如何在无 GC 的情况下保证内存安全？**
2. **如何在编译期消除整类并发 bug（而非运行时检测）？**
3. **如何用类型系统编码程序的不变量，让非法状态在编译期就不可构造？**
4. **如何提供高级抽象同时保持零运行时开销？**

这些问题的答案构成了 Rust 独有的设计范式。它们不是"另一种实现方式"，而是**根本不同的抽象维度**。

---

## 范式一：所有权即资源管理（Ownership as Resource Management）

### 问题域

所有语言都面临资源管理问题。Java 用 GC 管理内存（但文件、连接仍需手动关闭），Go 用 GC + defer。Rust 的选择是：**将资源管理编码到类型系统中**。

### 核心洞察：线性类型系统的工程化

Rust 的所有权系统本质上是**线性类型理论（Linear Type Theory）**的工程实现：

- **线性**：每个值必须有且只有一个 owner
- **仿射（Affine）**：值可以被使用一次（move）或零次（drop）
- **借用**：在不转移所有权的情况下临时访问

这不是"更好的指针管理"，而是**将资源的生命周期编码为类型约束**。

```rust
// 关键洞察：String 不是"字符串对象"，而是"堆内存资源的所有权凭证"
fn main() {
    let s1 = String::from("hello"); // s1 获得了堆内存资源的所有权
    let s2 = s1;                     // 所有权转移（move），s1 失效

    // println!("{}", s1); // 编译错误：s1 的值已被移动
    // 编译器在类型层面追踪：s1 不再拥有有效的资源凭证

    println!("{}", s2); // s2 是唯一的 owner，负责在作用域结束时释放内存
}
```

### 为什么不是 GC？

| 维度 | Rust 所有权 | Java GC | Go GC |
|------|------------|---------|-------|
| 释放时机 | 编译期确定（作用域结束） | 运行时 GC 决定 | 运行时 GC 决定 |
| 延迟保证 | 无 | 无（STW 停顿） | 无（STW 停顿） |
| 非内存资源 | 自动管理（RAII） | try-finally/try-with-resources | defer |
| 运行时开销 | 零 | 标记-清除/整理开销 | 并发标记开销 |
| 线程安全 | 编译期保证 | 依赖同步原语 | 依赖同步原语 |

### 超越内存：所有权作为通用资源管理范式

```rust
use std::fs::File;
use std::io::{Write, Result};

// File 的所有权语义：谁拥有 File，谁就拥有底层的文件描述符
fn write_log(file: File, msg: &str) -> Result<File> {
    // file 被 move 进函数
    let mut file = file;
    writeln!(file, "{}", msg)?;
    Ok(file) // 所有权返回给调用者
}

fn main() -> Result<()> {
    let file = File::create("app.log")?;

    // file 被 move 进 write_log，然后所有权返回
    let file = write_log(file, "Application started")?;
    let file = write_log(file, "Processing request")?;

    // file 在这里自动关闭——不是因为我们调用了 close()，
    // 而是因为 file 的 owner 离开了作用域，Drop trait 触发关闭
    Ok(())
}
```

**Java/Go 对比**：
- Java：`try (FileOutputStream f = new FileOutputStream("app.log")) { ... }` — 需要显式的 try-with-resources 语法
- Go：`defer file.Close()` — 容易忘记，且 Close 的错误经常被忽略
- Rust：资源关闭是**所有权的必然结果**，无法忘记

### 高级模式：所有权作为能力（Capability）

```rust
// 用所有权表达"能力"：只有持有 Token 才能执行某些操作
struct DatabaseToken;

struct DatabaseConnection {
    _token: DatabaseToken, // 私有字段，外部无法构造
}

impl DatabaseConnection {
    // 只有 DatabaseToken 的 owner 才能创建连接
    pub fn new(token: DatabaseToken) -> Self {
        DatabaseConnection { _token: token }
    }

    pub fn query(&self, sql: &str) -> Vec<String> {
        vec![format!("Result of: {}", sql)]
    }
}

// Token 只能从受信任的模块获取
pub fn acquire_db_token() -> DatabaseToken {
    DatabaseToken
}

fn main() {
    let token = acquire_db_token();
    let conn = DatabaseConnection::new(token);
    // token 已被 move，无法再次创建连接
    // let conn2 = DatabaseConnection::new(token); // 编译错误！

    let results = conn.query("SELECT * FROM users");
    println!("{:?}", results);
}
```

---

## 范式二：Send/Sync — 并发安全的类型级编码

### 问题域

并发 bug（数据竞争、死锁）是系统编程的噩梦。Java 用 `synchronized`、`volatile`、`java.util.concurrent`。Go 用 channel 和 `go` 关键字。Rust 的选择是：**在类型系统中编码线程安全性**。

### 核心洞察：标记 Trait 作为安全契约

Rust 用两个**空 trait**（没有方法的 trait）来定义类型的并发属性：

```rust
// std::marker 中的定义（简化）
pub unsafe auto trait Send {}
pub unsafe auto trait Sync {}
```

- **`Send`**：类型可以安全地**转移所有权**到另一个线程
- **`Sync`**：类型可以安全地**被多个线程同时引用**（即 `&T` 是 `Send`）

**关键设计**：这些 trait 是 **auto trait**，编译器自动为类型实现——除非类型包含非 Send/非 Sync 的字段。

```rust
use std::rc::Rc;
use std::sync::Arc;

fn main() {
    // Rc<T> 不是 Send：引用计数操作是非原子的
    let rc = Rc::new(42);
    // std::thread::spawn(move || {
    //     println!("{}", rc); // 编译错误：Rc<i32> 不能在线程间安全传递
    // });

    // Arc<T> 是 Send + Sync：使用原子引用计数
    let arc = Arc::new(42);
    let arc2 = Arc::clone(&arc);
    std::thread::spawn(move || {
        println!("{}", arc2); // OK！Arc 是 Send
    });
}
```

### 为什么这是范式而非特性？

Send/Sync 的设计代表了一种**将安全属性编码到类型系统**的通用方法：

| 安全属性 | Rust 编码方式 | Java/Go 做法 |
|---------|-------------|-------------|
| 线程间传递所有权 | `T: Send` | 无编译期检查，运行时发现 |
| 线程间共享引用 | `T: Sync` | `volatile`、`final`，易出错 |
| 数据竞争 | 编译期阻止 | `synchronized`、锁，运行期保护 |
| 发送后使用 | 所有权系统阻止 | 无保护，可能并发修改 |

###  fearless concurrency 的实践

```rust
use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // Arc<Mutex<i32>>：线程安全的共享可变状态
    // - Arc：共享所有权（原子引用计数）
    // - Mutex：互斥访问（编译器不会让你在无锁的情况下访问内部数据）
    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
            // 锁在这里自动释放：MutexGuard 的 Drop 实现
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Result: {}", *counter.lock().unwrap());
}
```

**与 Java 对比**：
```java
// Java：编译器不会阻止你忘记 synchronized
class Counter {
    private int count = 0;
    // 忘记 synchronized → 数据竞争，但编译通过
    public void increment() { count++; }
}
```

**与 Go 对比**：
```go
// Go：通过 channel 推荐"通过通信共享内存"
// 但如果你直接共享内存，编译器不会阻止数据竞争
var counter int // 多个 goroutine 直接修改 → 数据竞争
```

### 自定义类型的 Send/Sync 控制

```rust
// 显式实现（unsafe）或拒绝实现
struct RawPointerWrapper(*mut u8);

// 我们"知道"这个类型可以安全地在线程间传递
unsafe impl Send for RawPointerWrapper {}

// 我们"知道"这个类型不能安全地共享引用
// 不实现 Sync，编译器就不会允许 Arc<RawPointerWrapper> 被多线程共享
```

---

## 范式三：类型状态模式（Type State Pattern）

### 问题域

许多对象有生命周期状态，且某些操作只在特定状态下合法。传统 OOP 在运行时检查状态：

```java
// Java：运行时检查
class Connection {
    enum State { DISCONNECTED, CONNECTING, CONNECTED }
    private State state = State.DISCONNECTED;

    public void send(String data) {
        if (state != State.CONNECTED) {
            throw new IllegalStateException("Not connected!");
        }
        // ...
    }
}
```

### Rust 方案：用类型参数编码状态

```rust
// 用泛型参数表示状态，状态是类型系统的一部分
struct Disconnected;
struct Connected;

struct Connection<State> {
    state: std::marker::PhantomData<State>,
}

impl Connection<Disconnected> {
    pub fn new() -> Self {
        Connection { state: std::marker::PhantomData }
    }

    pub fn connect(self) -> Connection<Connected> {
        println!("Connecting...");
        Connection { state: std::marker::PhantomData }
    }
}

impl Connection<Connected> {
    pub fn send(&self, data: &str) {
        println!("Sending: {}", data);
    }

    pub fn disconnect(self) -> Connection<Disconnected> {
        println!("Disconnecting...");
        Connection { state: std::marker::PhantomData }
    }
}

fn main() {
    let conn = Connection::new();
    // conn.send("hello"); // 编译错误！Connection<Disconnected> 没有 send 方法

    let conn = conn.connect();
    conn.send("hello"); // OK！Connection<Connected> 有 send 方法

    let conn = conn.disconnect();
    // conn.send("world"); // 编译错误！已断开
}
```

### 为什么这比运行时检查更好？

| 维度 | Rust 类型状态 | Java 运行时检查 |
|------|-------------|---------------|
| 错误发现时机 | 编译期 | 运行期（可能生产环境） |
| 测试负担 | 不需要测试"非法状态操作" | 需要覆盖所有状态错误路径 |
| 性能 | 零开销（PhantomData 不占用空间） | 运行时分支判断 |
| 文档 | 类型即文档 | 需要阅读文档或源码 |

### 真实案例：Rust 的构建器模式

```rust
// 用类型状态保证构建器的正确调用顺序
struct RequestBuilder<State> {
    url: Option<String>,
    method: Option<String>,
    _state: std::marker::PhantomData<State>,
}

struct NoUrl;
struct HasUrl;
struct HasMethod;

impl RequestBuilder<NoUrl> {
    pub fn new() -> Self {
        RequestBuilder { url: None, method: None, _state: std::marker::PhantomData }
    }

    pub fn url(self, url: &str) -> RequestBuilder<HasUrl> {
        RequestBuilder {
            url: Some(url.to_string()),
            method: self.method,
            _state: std::marker::PhantomData,
        }
    }
}

impl RequestBuilder<HasUrl> {
    pub fn method(self, method: &str) -> RequestBuilder<HasMethod> {
        RequestBuilder {
            url: self.url,
            method: Some(method.to_string()),
            _state: std::marker::PhantomData,
        }
    }
}

impl RequestBuilder<HasMethod> {
    pub fn send(self) {
        println!("Sending {} {}", self.method.unwrap(), self.url.unwrap());
    }
}

fn main() {
    // RequestBuilder::new().send(); // 编译错误！必须先调用 url()
    // RequestBuilder::new().url("https://api.example.com").send(); // 编译错误！必须先调用 method()

    RequestBuilder::new()
        .url("https://api.example.com")
        .method("GET")
        .send(); // OK！
}
```

---

## 范式四：零成本抽象与单态化（Zero-Cost Abstractions）

### 问题域

高级抽象通常有运行时开销：虚函数调用、装箱、反射。Java 的泛型擦除、Go 的 interface 动态分派都有运行时成本。Rust 的设计目标是：**你使用的高级抽象在运行时不比手写底层代码更慢**。

### 核心机制：单态化（Monomorphization）

```rust
// 泛型函数：编译期为每个具体类型生成一份代码
fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

fn main() {
    let a = max(3, 5);        // 编译器生成 max_i32
    let b = max(3.14, 2.71);  // 编译器生成 max_f64
    // 运行时没有类型参数，直接调用具体函数
}
```

### 迭代器链的完全内联

```rust
fn main() {
    let sum: i32 = (0..100)
        .map(|x| x * 2)
        .filter(|x| x % 3 == 0)
        .sum();

    // 编译后的机器码等价于：
    // let mut sum = 0;
    // for x in 0..100 {
    //     let doubled = x * 2;
    //     if doubled % 3 == 0 {
    //         sum += doubled;
    //     }
    // }
    // 没有虚函数调用，没有中间集合分配

    println!("{}", sum);
}
```

### 对比表

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 泛型实现 | 单态化（每个类型一份代码） | 类型擦除（运行时类型检查） | 无泛型（1.18 前）/ 单态化（1.18+） |
| trait/接口分派 | 静态分派默认（dyn 显式选择动态） | 动态分派（vtable） | 动态分派（itab） |
| 迭代器 | 零成本链式操作 | Stream API 有装箱开销 | 无内置迭代器链 |
| 闭包 | 内联优化 | lambda 对象分配 | 函数值，有分配 |

### 动态分派是显式选择

```rust
trait Drawable {
    fn draw(&self);
}

struct Circle;
struct Square;

impl Drawable for Circle { fn draw(&self) { println!("Circle"); } }
impl Drawable for Square { fn draw(&self) { println!("Square"); } }

fn render_static<T: Drawable>(item: &T) {
    // 静态分派：编译期确定调用哪个 draw，可内联
    item.draw();
}

fn render_dynamic(item: &dyn Drawable) {
    // 动态分派：运行时查 vtable，显式标注 dyn
    item.draw();
}

fn main() {
    let c = Circle;
    render_static(&c);  // 默认：零开销
    render_dynamic(&c); // 显式选择：灵活性换性能
}
```

---

## 范式五：生命周期参数化（Lifetime Parametricity）

### 问题域

引用/指针的有效期是程序正确性的核心问题。其他语言要么用 GC（Java/Go），要么任由程序员犯错（C/C++）。Rust 将引用的有效期**编码为类型参数**。

### 核心洞察：将时间维度引入类型系统

```rust
// 'a 是一个生命周期参数，代表"某个作用域"
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

fn main() {
    let s1 = String::from("long string");
    let result;
    {
        let s2 = String::from("xyz");
        result = longest(&s1, &s2);
        println!("{}", result); // OK：s2 在这里还有效
    }
    // println!("{}", result); // 编译错误！result 可能引用 s2，而 s2 已失效
}
```

### 生命周期省略规则

Rust 有**生命周期省略（Lifetime Elision）**规则，让常见模式不显式标注：

```rust
// 这两者是等价的
fn first_word(s: &str) -> &str { ... }
fn first_word<'a>(s: &'a str) -> &'a str { ... }
// 编译器自动推断：返回值的生命周期与输入相同
```

### 高级模式：生命周期作为 API 契约

```rust
// 解析器返回的引用必须不长于输入字符串
struct Parser<'input> {
    source: &'input str,
}

impl<'input> Parser<'input> {
    fn new(source: &'input str) -> Self {
        Parser { source }
    }

    // 返回的 &str 不会比 source 活得更长
    fn parse_token(&self) -> &'input str {
        &self.source[..5]
    }
}

fn main() {
    let input = String::from("hello world");
    let parser = Parser::new(&input);
    let token = parser.parse_token();
    drop(parser);
    println!("{}", token); // OK：token 的生命周期绑定到 input，不是 parser
}
```

---

## 范式六：过程宏 — 编译期 DSL 构造

### 问题域

元编程（代码生成代码）是许多框架的核心。Java 用注解处理器（编译期），Go 用代码生成工具（go generate，非编译期）。Rust 提供**过程宏**，在编译期操作 AST。

### 三种过程宏

```rust
// 1. Derive 宏：为 struct/enum 自动生成 trait 实现
#[derive(Debug, Clone, PartialEq)] // 这三个都是过程宏
struct Point { x: i32, y: i32 }

// 2. Attribute 宏：自定义属性
#[route(GET, "/users")] // 自定义属性宏
fn get_users() -> Vec<User> { ... }

// 3. Function-like 宏：类似声明式宏，但更强大
let sql = sql!(SELECT * FROM users WHERE id = ?); // 编译期 SQL 语法检查
```

### 与 Java/Go 的对比

| 维度 | Rust 过程宏 | Java 注解处理器 | Go generate |
|------|-----------|---------------|-------------|
| 执行时机 | 编译期 | 编译期 | 编译前（独立步骤） |
| 输入 | Token Stream（AST） | AST | 任意文本 |
| 输出 | Token Stream（新代码） | 生成新源文件 | 生成新源文件 |
| 类型安全 | 生成的代码参与类型检查 | 生成后参与类型检查 | 生成后参与类型检查 |
| 使用体验 | 声明式，无额外构建步骤 | 需配置处理器 | 需手动运行 go generate |

### 设计哲学：宏即编译期函数

Rust 过程宏不是文本替换（不像 C 宏），而是**操作 AST 的编译期函数**。这保证了：

1. 宏的输出必然语法正确
2. 宏参与类型检查
3. IDE 可以理解和展开宏

---

## 范式七：Result/Option + ? 运算符 — 显式错误处理作为类型系统

### 问题域

错误处理是程序设计的基本问题。Java 用异常（隐式控制流，堆栈展开），Go 用多返回值（显式但繁琐）。Rust 将错误处理**编码到类型系统**。

### 核心设计：错误是值，不是异常

```rust
use std::fs::File;
use std::io::{self, Read};

// 返回值显式标注可能失败：Result<T, E>
fn read_username_from_file() -> Result<String, io::Error> {
    let mut file = File::open("hello.txt")?; // ?：失败时提前返回 Err
    let mut username = String::new();
    file.read_to_string(&mut username)?;     // ?：同上
    Ok(username)
}

// 等价于：
fn read_username_verbose() -> Result<String, io::Error> {
    let mut file = match File::open("hello.txt") {
        Ok(f) => f,
        Err(e) => return Err(e),
    };
    // ...
    Ok(String::new())
}
```

### 为什么不是异常？

| 维度 | Rust Result | Java Exception | Go error |
|------|------------|---------------|---------|
| 错误路径 | 显式（类型系统） | 隐式（控制流跳转） | 显式（返回值） |
| 编译期检查 | 必须处理或传播 | 受检异常可选 | 可忽略 |
| 性能 | 无堆栈展开开销 | 堆栈展开昂贵 | 无堆栈展开 |
| 组合性 | ? 运算符、map、and_then | try-catch 块 | if err != nil |

---

## 设计决策总对比

| Rust 范式 | 解决的问题 | Java 方式 | Go 方式 | Rust 的核心洞察 |
|----------|----------|----------|--------|---------------|
| 所有权系统 | 内存/资源安全 | GC + try-finally | GC + defer | 资源管理即类型约束 |
| Send/Sync | 并发安全 | synchronized、JUC | channel、mutex | 线程安全即类型属性 |
| 类型状态 | 非法状态不可达 | 运行时检查/异常 | 运行时检查 | 状态即类型 |
| 零成本抽象 | 高级抽象的性能 | 泛型擦除、装箱 | interface 动态分派 | 抽象不应有运行时税 |
| 生命周期 | 垂悬引用 | GC | GC | 时间即类型参数 |
| 过程宏 | 元编程/DSL | 注解处理器 | go generate | AST 操作即编译期函数 |
| Result/Option | 错误处理 | 异常 | 多返回值 | 错误即值 |

---

## 运行示例

### 类型状态模式

```bash
cargo run -p typestate_pattern
```

### Send/Sync 并发示例

```bash
cargo run -p send_sync_concurrency
```
