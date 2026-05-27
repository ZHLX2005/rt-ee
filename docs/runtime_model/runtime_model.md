# Rust 有类加载机制吗？

## 设计背景与问题域

从 Java 背景转向 Rust 时，一个自然的问题是：**Rust 的类在哪里加载？ClassLoader 在哪里？**

这个问题背后是一整套对程序运行模型的预设：
- Java：源代码 → 字节码(.class) → **运行时由 ClassLoader 加载到 JVM** → JIT 编译为机器码
- 这个模型意味着**类型信息在运行时完整保留**，可以反射、可以动态加载、可以运行时生成代理类

Rust 彻底打破了这个预设。

### 核心问题

1. **Rust 编译后产出的不是字节码，而是原生机器码**。没有 .class 文件，自然没有类加载。
2. **Rust 不是基于类的语言**。`struct` + `impl` 的组合不等于 Java 的 `class`——没有继承体系，没有默认的虚方法表，没有对象头中的类型指针。
3. **Rust 的运行时几乎为空**。没有 GC 线程，没有 JIT 编译器，没有类加载器——只有你的代码直接运行在 CPU 上。

---

## 直接回答

> **Rust 没有类加载机制。**

这不是"实现不同"，而是"根本不存在"。Rust 程序的所有类型信息在编译期就已经完全确定，编译后的二进制文件中只保留机器码和必要的符号信息。运行时不存在：

| Java 运行时组件 | Rust 对应物 | 说明 |
|----------------|-----------|------|
| ClassLoader | **不存在** | 所有代码在链接时已解析 |
| .class 字节码 | **不存在** | 直接输出机器码 |
| 对象头（类型指针）| **不存在** | 普通结构体无运行时类型信息 |
| 反射元数据 | **极度有限** | 仅 `Any` + `TypeId`，无字段/方法遍历 |
| JIT 编译器 | **不存在** | 编译期优化完成，运行时无编译 |
| GC 线程 | **不存在** | 所有权系统管理内存 |

---

## Rust 的编译与链接模型

### 编译产物是什么？

```
Java:  .java → javac → .class (字节码) → JVM 加载 → JIT → 机器码
Rust:  .rs   → rustc → .rlib/.a/.so (机器码/对象文件) → 链接器 → 可执行文件
```

Rust 的编译产物在链接后就完成了**所有**代码生成工作。可执行文件中包含：
- 机器码（text 段）
- 全局数据（data/bss 段）
- 符号表（用于调试，运行时可选剥离）

**没有字节码，没有中间层，没有运行时解释器。**

### 链接模型：静态为主

Rust 默认使用**静态链接**：

```rust
// Cargo.toml
[dependencies]
serde = "1.0"
```

编译后，`serde` 的代码被静态链接进最终可执行文件。运行时不需要：
- 查找 .jar 文件
- 解析依赖版本
- 动态加载库

```bash
# Rust 默认输出一个独立的可执行文件
cargo build --release
# 产物：target/release/myapp（单文件，可直接运行）
```

对比 Java：
```bash
# Java 运行时依赖 CLASSPATH
java -cp "lib/*:target/classes" com.example.Main
# JVM 启动后才按需加载类
```

### 动态链接是可能的，但不是"类加载"

Rust 支持动态链接（`cdylib`、`dylib`），也可以通过 `dlopen`/`libloading` 在运行时加载动态库：

```rust
use libloading::{Library, Symbol};

fn main() {
    unsafe {
        let lib = Library::new("./plugin.so").unwrap();
        let func: Symbol<fn() -> i32> = lib.get(b"plugin_entry").unwrap();
        println!("Result: {}", func());
    }
}
```

但这和 Java 的类加载**有本质区别**：

| 维度 | Java ClassLoader | Rust dlopen |
|------|-----------------|-------------|
| 加载内容 | 字节码（类型定义 + 方法体）| 机器码（已编译的符号）|
| 运行时类型信息 | 完整保留（Class 对象）| 无（只有 C ABI 符号）|
| 类型安全 | 字节码验证器保障 | 程序员负责（unsafe）|
| 动态创建对象 | `Class.newInstance()` | 无（只能调用已知签名的函数）|
| 反射调用 | `Method.invoke()` | 无 |

Rust 的动态加载是**操作系统级别的动态库加载**，不是语言级别的类型系统扩展。

---

## Rust 的运行时"多态"能力

既然没有类加载，Rust 如何实现运行时多态？答案是：**显式、有限、有代价**。

### 默认：编译时确定一切

```rust
struct User { name: String }

impl User {
    fn greet(&self) { println!("Hello, {}", self.name); }
}

fn main() {
    let u = User { name: String::from("Alice") };
    u.greet(); // 编译后直接调用 User::greet，无查找开销
}
```

编译后，`u.greet()` 就是一条直接的 `call` 指令，目标地址在链接时就已确定。没有对象头、没有虚表查找、没有运行时类型检查。

### 显式请求运行时多态：dyn Trait

```rust
trait Drawable {
    fn draw(&self);
}

struct Circle { radius: f64 }
struct Rectangle { width: f64, height: f64 }

impl Drawable for Circle {
    fn draw(&self) { println!("Circle"); }
}

impl Drawable for Rectangle {
    fn draw(&self) { println!("Rectangle"); }
}

// 动态分发：运行时通过 vtable 查找
fn render_all(items: &[Box<dyn Drawable>]) {
    for item in items {
        item.draw(); // vtable 查找
    }
}
```

`&dyn Drawable` 是一个**胖指针**（16 bytes）：
- 数据指针（指向具体对象）
- vtable 指针（指向方法地址表）

**关键区别**：
- Java：每个对象引用默认携带类型信息，所有实例方法调用默认是虚调用
- Rust：只有显式使用 `dyn Trait` 时才产生 vtable，且 vtable 在**编译时**生成，运行时不会加载新的实现

### 极度有限的运行时类型信息：Any

```rust
use std::any::{Any, TypeId};

fn inspect(x: &dyn Any) {
    println!("TypeId: {:?}", x.type_id());

    // 只能检查是否是编译时已知的类型
    if let Some(s) = x.downcast_ref::<String>() {
        println!("String: {}", s);
    }
    // 无法遍历字段、无法获取方法列表、无法动态调用
}
```

对比 Java 反射：
```java
// Java：运行时获取完整的类元数据
Class<?> clazz = obj.getClass();
for (Field f : clazz.getDeclaredFields()) {
    System.out.println(f.getName() + ": " + f.get(obj));
}
Method m = clazz.getMethod("foo");
m.invoke(obj);
```

Rust 的 `Any` 只是一个**类型标识比较器**，不是反射系统。这是有意的设计——运行时元数据有内存和性能开销，Rust 选择不支付这个代价。

---

## 代码示例（带设计意图注释）

完整示例见 `lab/runtime_model/src/main.rs`。关键代码段：

```rust
// 编译时类型完全确定
let user = User { name: String::from("Alice"), age: 30 };
user.greet(); // 编译为直接调用 User::greet，无运行时查找

// 泛型单态化：每个具体类型生成独立代码
print_size(&42i32);     // 编译为 print_size::<i32>
print_size(&String::new()); // 编译为 print_size::<String>

// dyn Trait：显式的运行时多态
let shapes: Vec<Box<dyn Drawable>> = vec![
    Box::new(Circle { radius: 3.0 }),
    Box::new(Rectangle { width: 4.0, height: 5.0 }),
];
// &dyn Drawable 是胖指针（16 bytes），Java 对象引用也是胖指针（对象头 12-16B + 引用）
// 但 Rust 只有在显式使用 dyn 时才有这个开销

// 内存布局对比
println!("Size of &dyn Drawable: {} bytes", std::mem::size_of::<&dyn Drawable>());
// 输出 16（64位：8B数据指针 + 8B vtable指针）
```

### 编译器错误示例

Rust 编译器会阻止任何试图在运行时动态创建类型的行为：

```rust
// 错误：Rust 无法在运行时从字符串创建类型
let type_name = "String";
let obj: Box<dyn Any> = create_from_type_name(type_name); // 不可能！

// 错误：Rust 没有运行时类元数据，无法遍历方法
for method in user.get_methods() { ... } // 没有这种方法

// 错误：无法像 Java 那样动态代理
trait Foo { fn bar(&self); }
let proxy = create_proxy::<dyn Foo>(...); // 没有内置动态代理机制
```

---

## 设计决策对比表

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **编译产物** | 机器码 | 字节码(.class) | 机器码 |
| **运行时加载** | 无（除 dlopen 动态库）| ClassLoader 加载字节码 | 无（动态插件需插件机制）|
| **运行时类型系统** | 无（编译时确定）| 完整（Class 对象、反射）| 有限（interface + type assertion）|
| **对象头开销** | 0（普通 struct）| 12-16 bytes（类型指针 + 锁 + GC 标记）| 无（但 interface 有间接层）|
| **方法调用默认** | 静态分发（直接调用）| 虚调用（vtable 查找）| 直接调用（interface 时间接）|
| **运行时多态** | 显式 `dyn Trait` | 隐式（所有实例方法）| interface（隐式）|
| **反射能力** | 极度有限（Any/TypeId）| 完整（字段、方法、代理）| 有限（type switch、反射包）|
| **动态代码加载** | dlopen（unsafe，C ABI）| ClassLoader（安全，类型完整）| plugin 包（有限）|
| **运行时大小** | 无（或极小）| JVM（数十 MB）| runtime（GC + 调度器，数 MB）|
| **启动速度** | 极快（直接执行机器码）| 慢（JVM 初始化 + 类加载）| 快 |

---

## 为什么 Rust 这样设计？

### 1. 零成本抽象

Rust 的设计哲学是：**你不使用的东西，就不应该付出代价。**

Java 的每个对象都携带类型信息，因为运行时系统需要它（GC、反射、动态代理）。即使你的程序从不使用反射，你仍然支付了对象头的内存开销和虚调用的性能开销。

Rust 说：**如果你需要运行时多态，请显式使用 `dyn Trait`，并且你知道自己在支付胖指针和 vtable 查找的代价。如果你不需要，你的代码就是纯静态的，性能等同于 C。**

### 2. 编译时确定一切

Rust 把尽可能多的工作搬到编译期：
- 类型检查 → 编译期
- 内存安全验证 → 编译期（借用检查器）
- 方法解析 → 编译期
- 泛型展开 → 编译期（单态化）

这带来了确定性和性能：
- 没有运行时 GC 暂停
- 没有 JIT 编译的预热时间
- 没有类加载的延迟
- 方法调用是直接的 CPU 跳转指令

### 3. 安全性的来源不同

| 语言 | 安全机制 | 位置 |
|------|---------|------|
| Java | 字节码验证器 + GC + 异常 | 运行时 |
| Rust | 借用检查器 + 类型系统 | **编译期** |

Java 需要运行时类型信息来执行字节码验证（确保类型转换合法、数组访问安全等）。Rust 在编译期就已经证明了这些安全属性，运行时不需要保留类型信息来执行检查。

---

## 常见误解

### 误解 1："Rust 的 struct + impl 就是 class"

```rust
struct User { name: String }
impl User { fn greet(&self) { ... } }
```

**错误。** Rust 的 `struct` 只是数据的聚合，`impl` 只是为该类型定义方法的语法。没有继承、没有虚方法表（默认）、没有对象头。`User` 的内存布局就是一个 `String` 的大小，没有任何额外开销。

### 误解 2："Rust 有反射"

```rust
// 这不是反射，只是类型标识比较
if x.is::<String>() { ... }
```

Rust 的 `Any` 只能回答"这个值是否是编译时已知的某个类型"。它不能：
- 列出类型的所有字段
- 获取字段的值
- 列出类型的所有方法
- 动态调用方法

### 误解 3："Rust 不能动态加载代码"

Rust **可以**通过 `dlopen`/`libloading` 加载动态库，但这和 Java 的类加载**完全不同**：
- 加载的是已编译的机器码，不是字节码
- 没有类型安全检查（完全依赖 C ABI 和程序员）
- 加载的代码必须是编译时已知的接口（函数签名）
- 无法在加载后发现新的类型或实现新的 trait

### 误解 4："dyn Trait 和 Java Interface 一样"

```rust
fn draw(item: &dyn Drawable) // Rust：显式动态分发
void draw(Drawable item)     // Java：隐式动态分发
```

Java 中所有对象引用默认携带类型信息，接口方法调用默认通过 vtable。Rust 中绝大多数调用是静态的，`dyn` 是显式的、有限的例外。

---

## 运行

```bash
cargo run -p runtime_model
```

---

## 设计哲学总结

> **Rust 没有类加载机制，是因为 Rust 不需要运行时类型系统。**

Java 的类加载、反射、JIT 是一套完整的设计——它们共同支撑了 Java "一次编译，到处运行"和"运行时灵活"的能力，代价是运行时的内存占用和性能开销。

Rust 走了另一条路：**编译期做尽可能多的事情，运行时只做最小的工作。** 这带来了：
- 更小的二进制体积（无运行时）
- 更快的启动速度（无类加载、无 JIT 预热）
- 更可预测的性能（无 GC 暂停、无 JIT 去优化）
- 更低的运行时内存占用（无对象头、无类型元数据）

代价是：
- 编译时间更长（编译期做运行时的活）
- 无法运行时动态创建类型
- 无法运行时反射遍历结构
- 无法像 Java 那样灵活地动态加载和卸载代码

这不是"好"与"坏"的选择，而是**不同的工程权衡**。Java 追求运行时的灵活性和开发效率，Rust 追求运行时的确定性和性能上限。理解这个权衡，是理解 Rust 设计哲学的关键一步。
