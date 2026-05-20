# 泛型进阶：嵌套泛型、泛型安全与性能优化

## 设计背景与问题域

### 核心问题

泛型编程有三个关键问题：

| 问题 | 说明 |
|------|------|
| 嵌套泛型 | 泛型参数本身也是泛型（如 `Vec<Option<T>>`）如何工作？ |
| 泛型安全 | 泛型代码如何保证类型安全？ |
| 性能优化 | 静态分发 vs 动态分发的权衡？ |

---

## 嵌套泛型

### 什么是嵌套泛型？

嵌套泛型是指泛型参数本身也是泛型：

```rust
Vec<Option<i32>>       // Vec 的元素是 Option<i32>
HashMap<String, Vec<u8>> // HashMap 的 value 是 Vec<u8>
Result<Box<dyn Error>, io::Error> // Result 的 Err 是具体类型
```

### 嵌套的层数

```rust
// 一层泛型
Vec<i32>

// 两层嵌套
Vec<Option<i32>>

// 三层嵌套
Result<Option<Vec<String>>, Box<dyn Error>>
```

### 嵌套与 Trait Bounds

```rust
// 嵌套泛型也需要正确的 Trait Bounds
fn process<T>(vec: Vec<Option<T>>)
where
    T: Clone,
{
    for opt in vec {
        if let Some(val) = opt {
            println!("{:?}", val.clone());
        }
    }
}
```

---

## 泛型安全

### Rust 泛型的安全性保证

#### 1. 编译时类型检查

```rust
let v: Vec<i32> = vec![1, 2, 3];
// let s: Vec<String> = v; // 编译错误！类型不匹配
```

#### 2. 泛型擦除 vs 具体化

**Java 的类型擦除**：
```java
// 编译后泛型信息被擦除
List<String> list = new ArrayList<>();
// 运行时无法知道 T 是什么
```

**Rust 的泛型具体化（Monomorphization）**：
```rust
// 编译器为每种具体类型生成专用代码
let v1: Vec<i32> = vec![1, 2, 3];
let v2: Vec<String> = vec!["a".to_string(), "b".to_string()];
// v1 和 v2 是完全不同的类型
```

#### 3. Zero-Sized Types (ZST)

```rust
// 空结构体不占用空间
struct Empty;

// Vec<Empty> 不占用额外内存
let v: Vec<Empty> = vec![Empty, Empty, Empty];
println!("size: {}", std::mem::size_of::<Vec<Empty>>()); // 24 bytes (Vec 本身的大小)
```

#### 4. PhantomData：标记类型安全性

```rust
use std::marker::PhantomData;

struct PhantomPair<T, U> {
    first: T,
    last: U,
    _marker: PhantomData<(T, U)>,
}

impl<T, U> PhantomPair<T, U> {
    fn new(first: T, last: U) -> Self {
        PhantomPair {
            first,
            last,
            _marker: PhantomData,
        }
    }
}
```

**为什么需要 PhantomData？**
- 告诉编译器"这个类型拥有 T 和 U"
- 确保 Drop 的正确性
- 让编译器知道类型参数的使用

---

## 泛型性能优化

### 静态分发（Monomorphization）

#### 工作原理

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
    let nums = vec![1, 2, 3, 4, 5];
    let chars = vec!['a', 'b', 'c'];

    // 编译器生成两个专用函数：
    // largest::<i32> 和 largest::<char>
    println!("{}", largest(&nums));
    println!("{}", largest(&chars));
}
```

**编译后**：
```rust
// 编译器生成的具体化版本
fn largest_i32(list: &[i32]) -> &i32 { /* ... */ }
fn largest_char(list: &[char]) -> &char { /* ... */ }
```

#### 性能优势

| 分发方式 | 性能 | 代码大小 | 灵活性 |
|----------|------|---------|--------|
| 静态分发 | 最佳（无虚函数调用） | 增加（每种类型生成一份） | 低（编译时确定） |
| 动态分发 | 略有开销（vtable 查找） | 小（一份代码） | 高（运行时多态） |

### 动态分发（dyn Trait）

#### 何时使用

```rust
// 场景：需要存储不同类型的值
trait Printable {
    fn print(&self);
}

// 使用 dyn Trait：运行时多态
let items: Vec<Box<dyn Printable>> = vec![
    Box::new(42i32),
    Box::new("hello".to_string()),
];

for item in items {
    item.print();
}
```

#### 性能对比

```rust
// 静态分发：编译时确定类型
fn print_static(item: &impl Printable) {
    item.print();
}

// 动态分发：运行时查找
fn print_dynamic(item: &dyn Printable) {
    item.print(); // 需要 vtable 查找
}
```

### 性能优化策略

#### 1. 优先使用静态分发

```rust
// 优先这样写
fn process<T: Trait>(item: T) { }

// 而不是
fn process(item: &dyn Trait) { }
```

#### 2. 小型数据结构用 Stack

```rust
// 小型数据结构用泛型，编译器会优化到栈上
fn smallest<T: PartialOrd>(a: T, b: T) -> T {
    if a < b { a } else { b }
}

// 大型数据结构用 dyn Trait
fn print_all(items: &[&dyn Display]) {
    for item in items {
        println!("{}", item);
    }
}
```

#### 3. Trait Object 的内存布局

```rust
// dyn Trait 是胖指针
// &dyn Trait = 指针 + vtable 指针
// Size: 16 bytes (64-bit 系统)

struct FatPtr {
    data: *const (),      // 8 bytes
    vtable: *const (),   // 8 bytes
}
```

---

## 泛型约束进阶

### 多重约束

```rust
use std::fmt::{Display, Debug};

fn print_debug<T>(value: T)
where
    T: Display + Debug,
{
    println!("Display: {}", value);
    println!("Debug: {:?}", value);
}
```

### 关联类型约束

```rust
trait Container {
    type Item: Clone;
    fn get(&self, index: usize) -> Option<&Self::Item>;
}

impl<T: Clone> Container for Vec<T> {
    type Item = T; // Item 必须实现 Clone

    fn get(&self, index: usize) -> Option<&Self::Item> {
        self.get(index)
    }
}
```

### const 泛型

```rust
// 编译时可计算的泛型参数
fn matrix_multiply<T, const N: usize>(a: [[T; N]; N], b: [[T; N]; N]) -> [[T; N]; N] {
    // N 在编译时已知，可以做优化
    let mut result = [[a[0][0]; N]; N];
    // ...
    result
}

fn main() {
    // 编译器为 3x3 矩阵生成专用代码
    let a = [[1, 2, 3], [4, 5, 6], [7, 8, 9]];
    let b = matrix_multiply::<i32, 3>(a, a);
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **泛型实现** | Monomorphization（具体化） | Type Erasure（类型擦除） | 无泛型（用 interface{}） |
| **性能** | 静态分发最优 | 装箱/拆箱开销 | 无泛型，有接口调度开销 |
| **类型信息** | 编译时保留 | 运行时丢失 | 无 |
| **特化** | 支持（通过 PhantomData） | 不支持 | 不适用 |

### Java 的类型擦除问题

```java
// Java：类型信息在运行时丢失
List<String> list1 = new ArrayList<>();
List<Integer> list2 = new ArrayList<>();
// 运行时：list1 instanceof List == true
// 无法知道 list1 是 List<String>

// 导致的问题：
// - 泛型数组创建困难
// - instanceof 检查无法使用泛型参数
```

### Rust 的优势

```rust
// Rust：每种泛型参数组合都是独特类型
let v1: Vec<i32> = vec![1, 2, 3];
let v2: Vec<String> = vec!["a".to_string()];

// v1 和 v2 是完全不同的类型
// 编译器知道具体类型，可以做优化
```

---

## 常见错误与正确模式

### 错误 1：泛型约束不足

```rust
// 错误！T 可能没有实现 Clone
fn clone_first<T>(vec: Vec<T>) -> Option<T> {
    vec.first().cloned()
}
```

```rust
// 正确：添加约束
fn clone_first<T: Clone>(vec: Vec<T>) -> Option<T> {
    vec.first().cloned()
}
```

### 错误 2：泛型与生命周期混淆

```rust
// 错误！
fn first_str<'a, T>(s: &'a str) -> &'a T {
    // T 和生命周期 'a 无关
}
```

```rust
// 正确：明确生命周期关系
fn first_str<'a>(s: &'a str) -> &'a str {
    &s[0..1]
}
```

### 错误 3：过多使用 dyn Trait

```rust
// 错误！过度使用动态分发
fn process_all(items: Vec<Box<dyn Trait>>) {
    for item in items {
        item.do_something();
    }
}
```

```rust
// 正确：考虑静态分发
fn process_all<T: Trait>(items: Vec<T>) {
    for item in items {
        item.do_something();
    }
}
```

---

## 设计哲学

### 泛型的权衡

1. **代码大小 vs 性能**：Monomorphization 生成更多代码，但性能更好
2. **灵活性 vs 确定性**：动态分发更灵活，但有运行时开销
3. **安全性 vs 复杂性**：泛型约束让代码更安全，但学习曲线更陡

### Monomorphization 的代价

```rust
// 如果有 Vec<T> 和 Vec<U>，为每种组合生成代码
let v1: Vec<i32> = vec![1, 2, 3];
let v2: Vec<String> = vec!["a".to_string()];
let v3: Vec<u64> = vec![1, 2, 3];

// 编译器生成三份 Vec 的实现代码
// 代码大小增加，但运行更快
```

---

## 总结

| 概念 | 说明 |
|------|------|
| 嵌套泛型 | `Vec<Option<T>>` 等多层泛型结构 |
| Monomorphization | 编译时为每种类型生成专用代码 |
| dyn Trait | 运行时通过 vtable 进行分发 |
| PhantomData | 标记类型参数的所有权关系 |
| const 泛型 | 编译时可计算的泛型参数 |

**性能选择指南**：
- 需要最佳性能 → 静态分发 `T: Trait`
- 需要运行时多态 → 动态分发 `&dyn Trait`
- 大型数据结构 → dyn Trait（避免代码膨胀）
- 小型数据结构 → 静态分发

**核心洞察**：Rust 的泛型通过 Monomorphization 实现零成本抽象——既有泛型的灵活性，又有接近手写代码的性能。
