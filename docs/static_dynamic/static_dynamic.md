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

### 分发（Dispatch）的概念

**分发（Dispatch）**指的是编译器/运行时**决定调用哪个具体方法实现**的过程。它是面向对象和多态系统的核心机制。

#### 静态分发（Static Dispatch）

编译期就确定调用哪个函数，直接生成函数调用指令：

```rust
trait Draw { fn draw(&self); }
struct Circle;
impl Draw for Circle { fn draw(&self) { println!("circle"); } }

fn render<T: Draw>(item: T) {
    item.draw(); // 编译时确定：这里一定是 Circle::draw
}

render(Circle); // 编译器生成 render_circle 专用函数
```

**特点**：
- 编译期解析，零运行时开销
- 通过泛型的**具体化（Monomorphization）**实现：为每个具体类型生成一份专用代码
- 无法处理异质集合（`Vec` 里混存多种类型）

#### 动态分发（Dynamic Dispatch）

运行时才确定调用哪个函数，通过 vtable 间接查找：

```rust
fn render(item: &dyn Draw) {
    item.draw(); // 运行时：从 vtable 查找 draw 的地址再跳转
}

render(&Circle); // 编译器只生成一份代码，运行时通过 vtable 分派
```

**特点**：
- 运行期解析，有间接调用开销（1-2 次额外内存访问）
- 一份代码处理所有实现该 trait 的类型
- 支持异质集合：`Vec<Box<dyn Draw>>` 可以混存 Circle、Rectangle 等

#### 对比表

| 维度 | 静态分发 | 动态分发 |
|------|---------|---------|
| 解析时机 | 编译期 | 运行期 |
| 实现机制 | 泛型具体化 | vtable 查找 |
| 代码体积 | 每种类型一份代码 | 一份通用代码 |
| 运行时开销 | 零（直接 call） | 间接调用（2-3 次内存访问）|
| 内联优化 | 可以内联 | 无法内联 |
| 异质集合 | 不支持 | 支持 |

#### 为什么 Rust 默认偏好静态分发？

Rust 的设计哲学是**显式支付代价**。静态分发是默认（泛型），动态分发需要显式写 `dyn`——这与 Java 完全相反（Java 默认虚调用，final 才能避免）。

---

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

### 胖指针（Fat Pointer）的本质

Rust 中的 `dyn Trait` 引用和 slice 引用都不是普通指针，而是**胖指针（fat pointer）**。

#### 什么是胖指针？

胖指针由**两个机器字（word）**组成，在 64 位系统上是 16 字节：

```rust
// &dyn Trait 的内存布局
let item: &dyn Drawable = &circle;

// 实际内存表示（16 bytes）：
// [0..8):   数据指针 — 指向 Circle 实例的地址
// [8..16):  vtable 指针 — 指向 Drawable trait 的方法地址表
```

```
栈上的 item（16 bytes）：
┌─────────────────┬─────────────────┐
│   数据指针        │   vtable 指针    │
│  0x7fff...1000  │  0x55...2000    │
└─────────────────┴─────────────────┘
       │                  │
       ▼                  ▼
   ┌─────────┐      ┌─────────────────────────────┐
   │ Circle  │      │ vtable for Drawable         │
   │  {r:5}  │      │ [0]: type_info 指针          │
   └─────────┘      │ [1]: drop_in_place 函数指针  │
                    │ [2]: draw() 函数指针         │
                    └─────────────────────────────┘
```

对比普通指针（瘦指针）：
```rust
let ptr: &Circle = &circle; // 8 bytes，只有数据指针
```

#### 为什么需要胖指针？

因为 `dyn Trait` 的**具体类型在编译时未知**。编译器不知道 `item` 指向的是 `Circle` 还是 `Rectangle`，所以无法在编译时确定：
- `draw()` 方法的具体地址
- `drop()` 时该调用哪个类型的析构函数
- 类型的大小和对齐方式

vtable 在编译时为每个实现了该 trait 的具体类型生成，包含了运行时分派所需的所有信息。

#### Slice 引用也是胖指针

```rust
let arr = [1, 2, 3, 4, 5];
let slice: &[i32] = &arr[1..4]; // &[2, 3, 4]

// 内存布局（16 bytes）：
// [0..8):   数据指针 — 指向 arr[1] 的地址
// [8..16):  长度 — 3（元素个数）
```

Slice 引用需要胖指针的原因：编译时不知道 slice 的长度。`[i32]` 是一个**动态大小类型（DST）**，编译器无法在栈上分配固定大小的空间。

#### vtable 的内存空间详解

vtable 是实现动态分发的核心数据结构。理解它的**存储位置、生命周期和大小**对评估动态分发的真实开销至关重要。

**1. vtable 存放在哪里？**

```
可执行文件布局（编译后）：
┌─────────────────────────────────────┐
│ .text 段 — 代码                     │
│ .rodata/.rdata 段 — 只读数据        │ ◄── vtable 在这里
│ .data 段 — 可读写全局变量            │
│ .bss 段 — 未初始化全局变量           │
│ ...                                 │
└─────────────────────────────────────┘
```

vtable 存放在**只读数据段（.rodata / .rdata）**中：
- **编译期生成**：编译器为每个 `(类型, trait)` 组合生成一个 vtable，直接嵌入可执行文件
- **运行时只读**：vtable 在程序运行期间永远不会被修改，多个实例共享同一个 vtable
- **静态分配**：不需要运行时分配内存，不占用堆空间

**2. 一个类型有多少个 vtable？**

每个类型对**每个它实现的 trait** 都有一个独立的 vtable：

```rust
struct Circle { radius: f64 }

trait Draw { fn draw(&self); }
trait Area { fn area(&self) -> f64; }

impl Draw for Circle { ... }
impl Area for Circle { ... }

// Circle 有两个 vtable：
// - vtable for Draw (Circle, Draw)
// - vtable for Area (Circle, Area)
```

**3. vtable 的内存布局**

```
vtable for Drawable (Circle 实现):
┌──────────────────────────────┐
│ [0] type_info 指针            │ ──→ Circle 的类型元数据（用于 downcast）
│ [1] drop_in_place 函数指针    │ ──→ Circle 的析构函数
│ [2] size_of::<Circle>()      │ ──→ 类型大小（用于内存分配）
│ [3] align_of::<Circle>()     │ ──→ 类型对齐要求
│ [4] draw() 函数指针           │ ──→ Circle::draw 的地址
│ [5] 其他 trait 方法指针...     │
└──────────────────────────────┘
```

**大小计算**：
```
vtable 大小 = (trait 方法数量 + 头部字段数) × 指针大小(8 bytes)

例如：Drawable trait 有 1 个方法
      vtable 大小 = (1 + 4) × 8 = 40 bytes

如果 100 个类型实现 Drawable：
      总 vtable 开销 = 100 × 40 = 4,000 bytes（约 4 KB）
```

**4. vtable 的生命周期**

| 阶段 | 行为 |
|------|------|
| 编译期 | 编译器生成 vtable，写入目标文件的 .rodata 段 |
| 链接期 | 链接器合并重复的 vtable，确定最终虚拟地址 |
| 运行期 | vtable 随程序加载到内存，只读，共享 |
| 程序结束 | 随进程销毁，无需清理 |

**5. 与 Java 方法表的对比**

| 维度 | Rust vtable | Java 虚方法表 |
|------|-------------|---------------|
| 创建时机 | 编译期 | 类加载期 |
| 存储位置 | 可执行文件 .rodata 段 | JVM 方法区/元空间 |
| 数量 | 每 (类型, trait) 一个 | 每类一个（包含所有方法）|
| 运行时修改 | 不可修改 | 不可修改（但类加载是动态的）|
| 大小确定 | 编译期完全确定 | 类加载时根据继承链计算 |
| 查找开销 | 固定偏移（O(1)） | 固定偏移（O(1)） |

**关键洞察**：Rust 的 vtable 是纯编译期产物，不依赖运行时环境。这与 Java 的类加载机制形成鲜明对比——Java 的方法表在类加载时动态构建，而 Rust 的 vtable 在编译时就已固化到可执行文件中。

#### 胖指针与 Java 引用的对比

| 维度 | Rust `dyn Trait` | Java 对象引用 |
|------|-----------------|---------------|
| 指针大小 | 16 bytes（胖指针）| 8 bytes（普通指针）|
| 类型信息位置 | 指针本身携带 vtable 指针 | 堆上的对象头携带 Class 指针 |
| 每个对象开销 | 0（类型信息在指针里）| 12-16 bytes 对象头 |
| 方法查找 | vtable 索引（两次间接）| 对象头 → Class → 方法表 |
| 虚调用开销 | ~2-3 个额外内存访问 | ~3-4 个额外内存访问 |

**关键洞察**：Rust 把"类型信息"从堆上的对象头移到了栈上的指针里。这意味着：
- 堆对象本身没有运行时类型开销（零成本）
- 只有当你选择动态分发（`dyn`）时才支付胖指针的代价
- 静态分发时，指针只有 8 bytes，调用是直接的函数跳转

#### `Box<dyn Trait>` 也是胖指针

```rust
let boxed: Box<dyn Drawable> = Box::new(Circle { radius: 5.0 });

// Box<dyn Drawable> 在栈上也是 16 bytes：
// [0..8):  堆指针（指向堆上的 Circle）
// [8..16): vtable 指针
```

`Box<T>` 本身是一个智能指针（8 bytes），但当 `T` 是 `dyn Trait` 时，它扩展为胖指针（16 bytes）。堆上的数据布局和普通 `Box<Circle>` 相同——类型信息只在指针层面，不在堆数据上。

#### `*const dyn Trait`：裸胖指针

Rust 甚至有胖裸指针：
```rust
let raw: *const dyn Drawable = item;
// 也是 16 bytes，只是没有借用检查
```

这是 Rust 中极少数"指针大小不固定"的场景。绝大多数 Rust 指针（`&T`、`Box<T>`、`*const T`）都是 8 bytes，只有涉及 DST（动态大小类型）时才会变成胖指针。

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
