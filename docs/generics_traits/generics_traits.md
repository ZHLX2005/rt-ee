# 泛型与 Trait

## 设计背景与问题域

### 什么是 Trait？为什么 Rust 需要 Trait？

Trait 是 Rust 的**行为抽象**机制。核心问题：**如何定义"可以被某种方式使用的类型"？**

在 Java 中，这是 interface；在 Go 中，这是 structural typing（隐式接口）；在 C++ 中，这是抽象基类。Rust 的 Trait 与它们有深层联系，但也有本质区别。

**Rust Trait 解决的核心问题**：
1. **行为抽象**：定义"什么操作是可用的"
2. **静态分发**：泛型编译时展开（monomorphization），零运行时开销
3. **动态分发**：通过 trait object（类似 Java 的接口类型）运行时查找

---

## 抽象设计分析

### Trait 的本质：类型的行为契约

```rust
// 定义一个契约：可以被总结的类型
trait Summary {
    fn summarize(&self) -> String;
}
```

**这个契约的含义**：
- 任何实现 `Summary` 的类型，必须提供 `summarize` 方法的实现
- 编译器强制这一约束：没有实现 trait 就不能调用其方法

**对比 Java 接口**：
```java
interface Summary {
    String summarize();
}
// 相同概念：定义行为契约
```

**对比 Go 隐式接口**：
```go
type Summary interface {
    summarize() string
}
// 区别：Go 是结构性类型匹配（structural typing），Rust 是名义性类型匹配（nominal typing）
```

### Rust Trait 的独特设计

#### 1. Trait 可以有默认实现

```rust
trait Summary {
    fn summarize(&self) -> String {
        // 默认实现：简单返回空字符串
        String::from("(No summary)")
    }
}
```

**设计意图**：提供可扩展的默认行为，同时允许具体类型覆盖。这比 Java 接口更强大（Java 8 才有 default 方法）。

#### 2. Trait 可以带泛型参数

```rust
trait Container<T> {
    fn get(&self, index: usize) -> Option<&T>;
    fn len(&self) -> usize;
}
```

#### 3. Trait 可以作为约束（Bounds）

```rust
// T 必须实现 Display + Clone
fn print_and_clone<T: Display + Clone>(value: T) {
    println!("{}", value);
    let _ = value.clone();
}
```

---

## 静态分发 vs 动态分发

### 静态分发（Monomorphization）

```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        if item > largest {
            largest = item;
        }
    }
    largest
}

fn main() {
    let numbers = vec![1, 2, 3, 5, 4];
    println!("{}", largest(&numbers)); // 编译器生成 i32 版本的函数
}
```

**工作原理**：编译器在编译时为每种具体类型生成专用代码。
- `largest::<i32>`：专门为 i32 生成的函数
- `largest::<char>`：专门为 char 生成的函数

**优点**：无虚函数调用开销，性能极高。

### 动态分发（Trait Object）

```rust
// 使用 dyn关键字创建 trait object
fn notify(item: &dyn Summary) {
    println!("{}", item.summarize());
}
```

**工作原理**：
- 编译时不知道 `item` 的具体类型
- 运行时通过函数指针表（vtable）查找方法
- 类似 Java 的方法调度

**对比**：
| 分发方式 | 语法 | 运行时查找 | 性能 | 灵活度 |
|----------|------|-----------|------|--------|
| 静态 | `T: Trait` | 无 | 零开销 | 只能一种类型 |
| 动态 | `&dyn Trait` | vtable | 有开销 | 多类型统一 |

---

## Trait 进阶机制

### 关联类型（Associated Types）

```rust
trait Iterator {
    type Item; // 关联类型：实现者定义的具体类型

    fn next(&mut self) -> Option<Self::Item>;
}

struct Counter {
    count: usize,
}

impl Iterator for Counter {
    type Item = usize; // 具体指定 Item 类型

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < 5 {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}
```

**为什么需要关联类型？**
- 如果用泛型参数：`trait Iterator<T>`，调用者必须指定 `Iterator<String>`
- 使用关联类型：`trait Iterator`，调用者通过 `Counter::Item` 获取类型
- 好处：**一个类型只能实现一次该 trait**（不能用同一迭代器同时产生 i32 和 String）

### Trait 限定（where 子句）

```rust
// 泛型约束的另一种语法，更清晰
fn some_function<T, U>(t: &T, u: &U) -> i32
where
    T: Display + Clone,
    U: Clone + Debug,
{
    // ...
}
```

### Trait 继承

```rust
trait Printable: Display {
    fn print(&self);
}

impl Printable for i32 {
    fn print(&self) {
        println!("{}", self);
    }
}
// i32 必须先实现 Display，才能实现 Printable
```

### blanket implementation（全覆盖实现）

```rust
// 所有实现了 Clone 的类型，自动实现 ToString
impl<T: Clone> ToString for T {
    fn to_string(&self) -> String {
        // ...
    }
}
```

**设计意图**：标准库为所有满足条件的类型提供通用实现，无需每个类型单独 impl。

---

## 与 Java/Go 的深度对比

| 维度 | Rust Trait | Java Interface | Go Interface |
|------|------------|----------------|--------------|
| **类型匹配** | 名义性（nominal） | 名义性 | 结构性（structural） |
| **默认实现** | 支持 | Java 8+ 支持 | 不支持 |
| **关联类型** | 支持 | 不支持 | 不支持 |
| **泛型约束** | where 子句 | extends | 无（隐式） |
| **static 方法** | 不支持 | 支持 | 不支持 |
| **常量字段** | 不支持 | 支持 | 不支持 |
| **实现多个** | 任意数量 | 任意数量 | 任意数量 |

### Rust Trait 的独特优势

1. **零成本抽象**：静态分发时无虚函数开销
2. **一致性约束**：关联类型防止同一类型的歧义实现
3. **默认实现**：减少样板代码
4. **Orphan Rule**：防止第三方 crate 为无关类型实现你的 trait（保证一致性）

### Rust Trait 的限制

```rust
// Rust 不允许为外部类型实现外部 trait
// 这叫做 Orphan Rule
impl ExternalTrait for ExternalType { } // 错误！
// 只能满足以下之一：
// - 为你的类型实现别人的 trait
// - 为别人的类型实现你的 trait
```

**对比 Java**：可以在任何类型上实现任何接口（导致方法污染）。

---

## 核心规则

### 实现 Trait 的规则

1. **不能为自身实现已有的 trait**：
   ```rust
   impl ToString for String { } // 错误！标准库已实现
   ```

2. **Orphan Rule**：见上

3. **trait 方法调度**：
   - 静态分发：`fn foo<T: Trait>(x: T)`
   - 动态分发：`fn foo(x: &dyn Trait)`

4. **Trait Bounds 的推导**：
   ```rust
   fn foo<T: Trait>(x: T) { x.method(); }
   // 编译器确保 x 一定实现了 method()
   ```

---

## 代码示例

### 示例 1：默认实现与覆盖

```rust
trait Describable {
    fn describe(&self) -> String {
        String::from("An object")
    }
}

struct Dog {
    name: String,
}

struct Cat;

impl Describable for Dog {
    fn describe(&self) -> String {
        format!("Dog named {}", self.name)
    }
}

impl Describable for Cat {} // 使用默认实现

fn main() {
    let dog = Dog { name: String::from("Buddy") };
    let cat = Cat;

    println!("{}", dog.describe()); // "Dog named Buddy"
    println!("{}", cat.describe()); // "An object"（默认实现）
}
```

### 示例 2：泛型约束与 where 子句

```rust
use std::fmt::{Display, Debug};

trait Print {
    fn print(&self);
}

impl<T: Display + Debug> Print for T {
    fn print(&self) {
        println!("Display: {}, Debug: {:?}", self, self);
    }
}

fn print_all<T>(items: &[T])
where
    T: Print,
{
    for item in items {
        item.print();
    }
}
```

### 示例 3：关联类型的实际应用

```rust
trait Graph {
    type N;
    type E;

    fn new() -> Self;
    fn add_node(&mut self, n: Self::N);
    fn add_edge(&mut self, from: Self::N, to: Self::N, e: Self::E);
}
```

---

## 设计哲学

### Trait 是 Rust 抽象的基石

1. **组合优于继承**：Rust 没有类继承，通过 trait 组合行为
2. **接口隔离**：只暴露必需的方法
3. **零成本抽象**：静态分发时没有运行时开销

### 为什么 Rust 选择名义性 trait 而非结构性的 Go 接口？

Go 的隐性接口虽然灵活，但：
- 无法在编译时确定一个类型实现了哪些接口
- 容易出现"意外实现"（第三方库更新后突然实现了一个你的类型）

Rust 的名义性 trait：
- 显式声明 `impl Trait for Type`
- 编译器强制检查
- Orphan Rule 防止意外冲突

---

## 运行

```bash
cargo run -p generics_traits
```
