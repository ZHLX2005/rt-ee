# 静态与动态语言的体现

## 设计背景与问题域

### 核心问题

编程语言在类型检查时机的设计上存在两种范式：

| 范式 | 类型检查时机 | 代表语言 | 优势 | 代价 |
|------|-------------|---------|------|------|
| 静态类型 | 编译时 | Rust, Java, C++ | 性能最优、运行时无类型开销 | 灵活性较低 |
| 动态类型 | 运行时 | Python, JavaScript, Ruby | 灵活性高 | 运行时类型检查开销 |

**Rust 的立场**：纯静态类型语言，但通过 `dyn Trait` 提供有限的运行时多态能力。

---

## Rust 的静态特性

### 编译时类型检查

Rust 是纯静态类型语言——所有类型在编译时确定：

```rust
let x: i32 = 42;
let s: String = String::from("hello");

// 编译错误！类型不匹配
// x = s; // Error: mismatched types
```

**为什么 Rust 选择静态？**
- 零成本抽象：没有运行时类型标签
- 内存布局在编译时确定，无额外指针查找
- 编译器可以做激进优化

### 泛型的具体化（Monomorphization）

Rust 的泛型在编译时生成专用代码：

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
    let nums = vec![1, 2, 3];
    let chars = vec!['a', 'b', 'c'];

    // 编译后生成两个独立函数：
    // largest_i32 和 largest_char
    println!("{}", largest(&nums));
    println!("{}", largest(&chars));
}
```

**具体化的代价**：
```rust
// 如果使用多种类型组合，代码大小会增加
let v1: Vec<i32> = vec![1, 2, 3];
let v2: Vec<String> = vec!["a".to_string()];
let v3: Vec<u64> = vec![1, 2, 3];
// 编译器生成三份 Vec<T> 的实现代码
```

### 栈分配的确定性

```rust
fn process() {
    // 栈上分配，大小在编译时确定
    let x: (i64, i64, i64) = (1, 2, 3);
    let s: [u8; 64] = [0; 64];

    // 编译器知道确切内存布局
    println!("{}", std::mem::size_of_val(&x)); // 24 bytes
    println!("{}", std::mem::size_of_val(&s)); // 64 bytes
}
```

---

## Rust 的动态特性

### dyn Trait：运行时多态

Rust 通过 `dyn Trait` 提供运行时多态能力：

```rust
trait Printable {
    fn print(&self);
}

struct Foo(i32);
struct Bar(String);

impl Printable for Foo {
    fn print(&self) { println!("Foo({})", self.0); }
}

impl Printable for Bar {
    fn print(&self) { println!("Bar({})", self.0); }
}

fn main() {
    // 静态分发：编译时确定类型
    let foo = Foo(42);
    foo.print(); // 直接调用，无额外开销

    // 动态分发：运行时确定
    let items: Vec<Box<dyn Printable>> = vec![
        Box::new(Foo(1)),
        Box::new(Bar("hello".to_string())),
    ];

    for item in items {
        item.print(); // 通过 vtable 查找，略有开销
    }
}
```

**动态分发是有代价的**：

```rust
// 内存布局对比
// 静态分发：直接调用
Foo(42).print(); // 编译时确定，直接 call Foo_print

// 动态分发：胖指针 + vtable
// &dyn Printable = 指针(8B) + vtable指针(8B) = 16 bytes
// 调用时需要：指针 → vtable → 函数地址
```

### 有限反射：std::any

Rust 提供有限的运行时类型信息：

```rust
use std::any::{TypeId, Any};

fn main() {
    let x: i32 = 42;
    let s: String = String::from("hello");

    // TypeId：获取类型的唯一标识
    println!("i32: {:?}", TypeId::of::<i32>());
    println!("String: {:?}", TypeId::of::<String>());

    // 限制：无法在运行时动态创建任意类型
    // 无法像 Java 那样：Object obj = getAnyType()

    // 只能对已知类型进行类型查询
    fn print_if_string(x: &dyn Any) {
        if x.is::<String>() {
            println!("It's a String!");
        }
    }

    print_if_string(&s);
}
```

**Rust 反射的限制**：

| 反射能力 | Java | Rust |
|---------|------|------|
| 获取类型信息 | `obj.getClass()` | `TypeId::of::<T>()` |
| 运行时创建类型 | `Class.forName()` | 不可用 |
| 动态方法调用 | 反射 API | 不可用 |
| 类型转换 | `(Type) obj` | `dyn Any` + `is::<>()` |

---

## 与 Java/Go 的深度对比

### Java：全面运行时类型系统

```java
// Java 的类型在运行时保留
Object obj = "hello";  // 运行时知道这是 String
String s = (String) obj;  // 运行时类型转换检查

// 泛型类型擦除：运行时不知道 T 是什么
List<String> list1 = new ArrayList<>();
List<Integer> list2 = new ArrayList<>();
// 运行时：两者都是 raw List，无法区分
```

**Java 的代价**：
- 每个对象有类型标签（Class pointer）
- 装箱/拆箱有运行时检查开销
- 泛型需要类型擦除或自icorn

### Go：静态但简单

```go
// Go 也是静态类型，但更简单
var x interface{} = 42  // 运行时知道是 int
// 类型断言
s := x.(int)

// Go 的 interface{} 类似 dyn Any
// 但 Go 没有泛型（Go 1.18 之前）
```

### Rust 的设计选择

```rust
// Rust：静态为主，动态为辅
// 默认使用静态分发，性能最优
fn process<T: Trait>(item: T) { }  // 编译时展开

// 仅在需要运行时多态时才使用 dyn
fn process_dynamic(item: &dyn Trait) { }  // vtable 查找
```

**Rust 的权衡**：

| 场景 | 推荐方式 | 原因 |
|------|---------|------|
| 同质类型集合 | 静态分发 `Vec<T>` | 无额外开销 |
| 异质类型集合 | `Vec<Box<dyn Trait>>` | 需要运行时多态 |
| 性能关键路径 | 静态分发 | 避免 vtable 查找 |
| 插件系统 | `dyn Trait` | 运行时加载 |

---

## 类型系统的哲学

### 静态优先原则

```rust
// 优先静态分发
// 设计意图：性能最优，无运行时开销
fn sum<T: Add>(a: T, b: T) -> T {
    a + b
}

// 仅在必要时使用动态分发
// 设计意图：灵活性 > 性能
fn process_all(items: Vec<Box<dyn Trait>>) {
    for item in items {
        item.do_something();
    }
}
```

### 零成本抽象

```rust
// Rust 的抽象是零成本的
// 静态分发：性能 = 手写代码
// dyn Trait：性能 ≈ 虚函数调用

// 对比 Java：
// Java 的接口调用也是虚函数
// 但 Java 有 GC、装箱/拆箱等额外开销

// 对比 Python：
// Python 的属性访问有字典查找开销
// Rust 的结构体访问是直接内存偏移
```

### 确定性优于非确定性

```rust
// Rust：编译时确定所有类型
// 没有运行时类型决定论的"惊喜"

// Java：GC 可能在任意时刻运行
// Python：GIL 导致线程切换不确定

// Rust 的确定性：
// - 内存释放时机由作用域决定
// - 线程切换由 await/锁决定
// - 无隐藏的运行时开销
```

---

## 常见误解

### 误解 1：Rust 是动态类型语言

```rust
// 错误：认为 Rust 像 Python 一样灵活
let x = 42;      // Rust 推断为 i32
x = "hello";     // 编译错误！x 已经是 i32
```

### 误解 2：dyn Trait == Java Interface

```rust
// 相似但不同：
// Java Interface：所有实现都在运行时确定
// dyn Trait：显式标记需要运行时查找

// Java：默认就是动态分发
interface Printable { void print(); }

// Rust：需要显式使用 dyn
fn print_static(item: &impl Printable) { }  // 静态
fn print_dynamic(item: &dyn Printable) { }  // 动态
```

### 误解 3：没有泛型也能工作

```rust
// Rust 的类型系统依赖泛型
// 没有泛型意味着大量代码重复

// Java 1.4 的困境：每种类型都需要单独实现
// Rust 通过泛型避免这个问题，同时保持静态类型
```

---

## 总结

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **类型检查** | 编译时 | 运行时 | 编译时 |
| **泛型实现** | Monomorphization | Type Erasure | 无（interface{}） |
| **多态分发** | 静态+动态 | 运行时 | 接口 |
| **反射能力** | 有限 | 完整 | 有限 |
| **性能** | 最优 | 中等 | 高 |

**Rust 的设计哲学**：
- **静态为主**：编译时确定一切，追求零成本抽象
- **动态为辅**：仅在需要灵活性时使用 `dyn Trait`
- **显式优于隐式**：动态分发需要显式标记
- **确定性优于非确定性**：无 GC，无隐藏运行时开销

**核心洞察**：Rust 通过静态类型和具体化实现了"零成本抽象"——既有静态语言的性能，又有动态语言的灵活性，但灵活性需要显式请求。
