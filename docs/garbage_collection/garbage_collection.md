# Rust 与垃圾回收（GC）

## 设计背景与问题域

**Rust 没有垃圾回收器（GC）**。这是 Rust 最显著的设计特征之一，也是许多开发者从 Java/Go 转向 Rust 时最大的认知冲击。

但"没有 GC"不等于"手动管理内存"。Rust 选择了一条第三条路：**编译期自动内存管理**。理解这一点需要回答：

1. **为什么 Rust 不像 Java/Go 那样使用 GC？**
2. **Rust 用什么替代了 GC？**
3. **Rust 的引用计数（Rc/Arc）是 GC 吗？**
4. **什么时候需要在 Rust 中使用 GC 模式？**
5. **Rust 生态中有没有真正的 GC 库？**

---

## 为什么 Rust 没有 GC？

### GC 的三个固有成本

| 成本 | 说明 | Rust 的替代方案 |
|------|------|---------------|
| **暂停（Pause）** | GC 需要停止程序线程来扫描堆内存 | 无暂停：内存释放是确定性的 |
| **空间开销** | GC 需要额外的标记位、引用计数或分代区域 | 无额外空间：只有对象本身 |
| **缓存不友好** | GC 的标记-清除会随机访问内存，破坏 CPU 缓存 | 顺序释放：遵循创建顺序的逆序 |

### 设计哲学：将运行时成本搬到编译期

```java
// Java：运行时 GC 决定何时释放
String s = new String("hello");
s = null; // 内存不会立即释放，等待 GC 扫描
```

```rust
// Rust：编译器在编译时就确定了释放时机
let s = String::from("hello");
// s 在这里离开作用域，编译器自动插入 drop(s)
```

Rust 的权衡：**用更长的编译时间 + 更严格的语法规则，换取零运行时 GC 开销**。

---

## Rust 用什么替代了 GC？

### 核心机制：所有权 + 确定性析构

Rust 的内存管理不是手动的，而是**自动的、确定性的**：

```rust
fn process() {
    let data = vec![1, 2, 3, 4, 5]; // 堆内存分配
    println!("{:?}", data);
} // data 离开作用域，Vec::drop() 自动调用，堆内存立即释放
```

**不是手动管理**：你没有调用 `free()` 或 `delete`。
**不是 GC**：没有后台线程扫描内存。
**是编译器自动管理**：编译器在编译时就确定了每个值的创建和销毁位置。

### Drop trait：资源释放的通用契约

```rust
pub trait Drop {
    fn drop(&mut self);
}

// Vec<T> 的 Drop 实现
impl<T> Drop for Vec<T> {
    fn drop(&mut self) {
        // 1. 逐个调用元素的 drop
        // 2. 释放堆上的缓冲区
        // 3. 将内存归还给分配器
    }
}
```

**关键洞察**：Drop trait 不仅用于内存，还用于**任何资源**——文件、网络连接、锁、数据库事务。Rust 的"无 GC"实际上意味着"通用资源管理的统一抽象"。

---

## Rc/Arc：Rust 中最接近 GC 的机制

### 引用计数不是 GC

```rust
use std::rc::Rc;

let data = Rc::new(vec![1, 2, 3]);
let data2 = Rc::clone(&data); // 引用计数 +1
let data3 = Rc::clone(&data); // 引用计数 +1
// data, data2, data3 都指向同一个堆内存

// 当所有 Rc 都离开作用域，引用计数归零，内存立即释放
```

**Rc 与 GC 的本质区别**：

| 维度 | Rc/Arc | GC |
|------|--------|-----|
| 回收时机 | 引用计数归零时**立即**释放 | 不确定，等待 GC 扫描 |
| 循环引用 | **会导致内存泄漏** | 自动检测并回收 |
| 运行时开销 | 原子操作（Arc）或普通递增（Rc） | 后台线程 + 扫描整个堆 |
| 暂停 | 无 | 有（stop-the-world） |

### 为什么 Rc 不处理循环引用？

```rust
use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
}

fn main() {
    let a = Rc::new(RefCell::new(Node { value: 1, next: None }));
    let b = Rc::new(RefCell::new(Node { value: 2, next: None }));

    a.borrow_mut().next = Some(Rc::clone(&b));
    b.borrow_mut().next = Some(Rc::clone(&a));
    // a 和 b 的引用计数都是 2
    // 它们离开作用域时，引用计数降到 1（不是 0）
    // 内存泄漏！
}
```

**GC 的优势**：Java/Go 的 GC 通过**可达性分析**（从根对象出发遍历引用图）检测循环引用，不受引用计数限制。

**Rust 的解法**：`Weak<T>`

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    value: i32,
    parent: Option<Weak<RefCell<Node>>>, // Weak 不增加引用计数
    children: Vec<Rc<RefCell<Node>>>,
}

fn main() {
    let root = Rc::new(RefCell::new(Node {
        value: 1,
        parent: None,
        children: vec![],
    }));

    let child = Rc::new(RefCell::new(Node {
        value: 2,
        parent: Some(Rc::downgrade(&root)), // Weak 引用
        children: vec![],
    }));

    root.borrow_mut().children.push(Rc::clone(&child));
    // root.strong_count = 1 + 1 (来自 child 的 children vec) = 2
    // child.strong_count = 1 + 1 (来自 root 的 children vec) = 2
    // 但 child.parent 是 Weak，不增加 strong_count

    // 当 root 和 child 的 Rc 变量离开作用域：
    // root strong_count: 2 - 1 = 1，但 child 还持有 root 的 Rc
    // 当 child 被 drop，root strong_count 降为 1
    // 然后 root 被 drop...
    // 实际上需要更仔细的分析，但 Weak 确实打破了循环
}
```

---

## 什么时候 Rust 中需要 GC 模式？

### 场景 1：复杂的共享所有权图

当你有一个高度互联的数据结构（如图、复杂对象网络），且无法预先确定所有权关系时，引用计数 + Weak 可能变得繁琐：

```rust
// 复杂的图结构，每个节点可能有多个入边和出边
// 用 Rc + Weak 管理需要为每个引用决定是强引用还是弱引用
// 这种情况下，GC 可能更简单
```

### 场景 2：长时间运行的程序中的缓存

缓存需要"在内存压力下自动释放旧对象"——这正是 GC 的强项。Rust 中需要手动实现或引入 GC 库。

### 场景 3：互操作（FFI）

与 GC 语言（如 JavaScript、Python）交互时，可能需要托管这些语言的 GC 对象。

---

## Rust 生态中的 GC 库

虽然标准库没有 GC，但生态中有第三方实现：

### gc crate

```rust
use gc::{Gc, Trace, Finalize};

#[derive(Trace, Finalize)]
struct Node {
    value: i32,
    next: Option<Gc<Node>>,
}

fn main() {
    let a = Gc::new(Node { value: 1, next: None });
    let b = Gc::new(Node { value: 2, next: Some(Gc::clone(&a)) });
    // Gc 自动处理循环引用！
}
```

**gc crate 的设计**：
- 基于**引用计数 + 周期检测**（类似 Python 的 GC）
- 当引用计数无法归零时，后台检测循环引用并回收
- 需要类型实现 `Trace` trait（标记哪些字段包含 `Gc` 引用）

### shredder crate

更先进的 GC 库，使用**并发标记-清除**算法，支持多线程。

### 为什么标准库没有 GC？

因为一旦引入 GC，就会带来：
1. 运行时暂停（即使是增量 GC）
2. 与 `Box`、`Rc` 的语义冲突（谁负责释放？）
3. 与 `unsafe` 代码的交互复杂性

Rust 的设计选择是：**标准库无 GC，需要时由第三方库提供**。

---

## 深度对比：Rust vs Java vs Go

### 内存管理总览

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 核心机制 | 所有权 + 确定性析构 | GC（G1/ZGC/Shenandoah） | GC（三色标记） |
| 释放时机 | 编译期确定 | 运行时不确定 | 运行时不确定 |
| 暂停 | **无** | 有（毫秒级到秒级） | 有（微秒级到毫秒级） |
| 内存开销 | 对象本身 | 对象 + GC 元数据 | 对象 + GC 元数据 |
| 循环引用 | 需 Weak 手动处理 | 自动处理 | 自动处理 |
| 缓存友好性 | 高（确定性释放） | 中（GC 可能随机访问） | 中 |
| 实时系统适用性 | **适合** | 不适合 | 部分适合 |

### Java GC 的演进

| GC 算法 | 特点 | 暂停时间 |
|--------|------|---------|
| Serial GC | 单线程，简单 | 长 |
| Parallel GC | 多线程并行 | 较长 |
| G1 GC | 分区 + 增量回收 | 可预测（默认） |
| ZGC | 并发标记-整理 | **亚毫秒级** |
| Shenandoah | 并发压缩 | **亚毫秒级** |

Java 的 ZGC 和 Shenandoah 已经将暂停时间降低到亚毫秒级，但**仍然存在**。对于需要严格实时性的系统（游戏引擎、高频交易、嵌入式），GC 暂停是不可接受的。

### Go GC 的设计

Go 使用**三色标记 + 混合写屏障**：
- 标记阶段：与程序并发执行
- 清除阶段：与程序并发执行
- 只有**扫描根对象**时需要短暂暂停（通常 <1ms）

Go 的 GC 设计目标是**低延迟**，但代价是：
- 更高的 CPU 开销（并发标记消耗 CPU 周期）
- 内存碎片（不压缩堆）
- 无法完全避免暂停

---

## 设计哲学总结

### Rust 的立场：GC 是一种权衡，不是必然

GC 解决了"忘记释放内存"的问题，但引入了新的问题：
1. **非确定性性能**：不知道 GC 何时会暂停程序
2. **空间-时间权衡**：要么耗内存（降低 GC 频率），要么耗 CPU（频繁 GC）
3. **资源泛化困难**：GC 只管理内存，不管理文件、连接、锁等其他资源

Rust 的替代方案：
1. **编译期自动管理**：通过所有权和 Drop trait，内存和其他资源统一自动管理
2. **确定性性能**：资源释放的时机是编译期确定的，没有运行时惊喜
3. **零运行时开销**：没有后台线程，没有标记-清除，没有内存屏障

### 什么时候选择 GC？

| 场景 | 推荐 | 原因 |
|------|------|------|
| 快速原型开发 | Java/Go/Python | GC 减少认知负担 |
| 复杂的共享所有权图 | Java/Go 或 Rust + GC crate | 引用计数难以管理 |
| 实时系统（游戏、交易） | **Rust** | GC 暂停不可接受 |
| 嵌入式/系统编程 | **Rust** | GC 运行时太大 |
| 长生命周期缓存 | Java/Go 或 Rust + 手动策略 | GC 自动回收旧对象 |

---

## 运行示例

```bash
cargo run -p garbage_collection
```
