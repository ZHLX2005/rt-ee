# Rust 的内存回收机制

## 设计背景与问题域

### 核心问题：Rust 没有 GC，如何避免内存泄漏？

传统上，程序员认为内存管理只有两种方式：
1. **手动管理**（C/C++）：malloc/free, new/delete
2. **垃圾回收**（Java/Go, Python, JavaScript）：GC 自动追踪引用并回收

Rust 选择了第三条路：**所有权 + 确定性析构（Deterministic Destruction）**。

**关键概念澄清**：Rust 确实没有 GC（garbage collector），但 Rust **有自动内存管理**。区别在于：
- GC 通过追踪"哪些对象仍然被引用"来判断是否需要回收（**推测式回收**）
- Rust 通过"谁拥有这个对象"来判断何时回收（**确定性析构**）

---

## 栈 vs 堆：Rust 的内存布局

### 栈（Stack）：固定大小的值

栈的特点：**先进后出，内存分配和释放是自动的**。

```rust
fn main() {
    let x = 5;          // i32 是 Copy 类型，直接在栈上分配
    let y = x;          // 拷贝 x 的值，x 和 y 都在栈上
    println!("{}", x);  // x 仍然有效，因为 i32 是 Copy
} // x 和 y 在函数结束时自动弹出栈，无需任何操作
```

**Rust 的栈行为与 C/C++ 类似**：固定大小的值直接分配在栈上，函数返回时自动释放。但 Rust 的借用检查器确保你不会 use-after-free。

### 堆（Heap）：动态大小的值

堆的特点：**手动分配，运行时决定大小**。

```rust
fn main() {
    let s = String::from("hello");
    //       ↑ String 动态分配在堆上
    // String 的结构（在栈上）：ptr（指向堆数据）, len, capacity
    // "hello" 这个字符串数据：分配在堆上

    let s2 = s; // 移动语义：s 的所有权转移到 s2
    // 此时：
    // - s 的 ptr 变为无效（所有者变成 s2）
    // - s2 持有指向堆上 "hello" 的指针
    // - 只有一个 owner（s2），不会 double-free

    println!("{}", s2);
} // s2 离开作用域，String::drop() 被调用，堆内存被释放
```

**为什么 String 不是 Copy？**
```rust
// 如果 String 是 Copy：
let s1 = String::from("hello");
let s2 = s1; // 假设这是拷贝
// 现在 s1 和 s2 都指向同一个堆数据
// s1 和 s2 离开作用域时都会调用 drop
// 结果：double-free（同一块内存被释放两次）

// 所以 String 是 Move 类型，确保只有一个 owner
```

---

## 核心机制：Drop trait 与确定性析构

### Drop trait：资源释放的契约

```rust
// Drop trait 的定义（简化）
pub trait Drop {
    fn drop(&mut self);
}

// String 实现了 Drop：
impl Drop for String {
    fn drop(&mut self) {
        // 调用 glibc 的 free() 或 jemalloc 等分配器
        // 释放堆上的内存
    }
}
```

**Rust 在每个作用域结束时自动调用 drop**：

```rust
fn main() {
    let s = String::from("hello");
    // ... 使用 s ...
} // 编译器在这里自动插入 drop(s)
```

### 确定性析构的执行顺序

```rust
fn main() {
    let s1 = String::from("first");
    let s2 = String::from("second");
    println!("s1 = {}, s2 = {}", s1, s2);
} // 析构顺序：先 s2，后 s1（栈的 LIFO 顺序）
```

**对比 Java 的 GC**：
```java
public class Main {
    public static void main(String[] args) {
        String s = new String("hello");
        // s 离开作用域时不会立即释放
        // GC 会在不确定的时机回收这块内存
        // 这就是"非确定性析构"
    }
}
```

---

## 内存回收的技术细节

### 栈内存：自动释放，无需回收

```rust
fn foo() {
    let x: i32 = 42;        // 4 bytes，分配在栈上
    let arr: [i32; 1000] = [0; 1000]; // 4000 bytes，栈上
    // 函数返回时，整个栈帧被弹出
    // 栈上所有数据同时被"释放"
    // 无需逐个回收，这是栈的天生特性
}
```

栈内存的释放**不是逐个对象**的，而是通过弹出整个栈帧实现的。

### 堆内存：通过 Drop trait 精确释放

```rust
fn bar() {
    let b = Box::new([0i32; 1000]); // Box 在栈上，但数据在堆上
    // 实际上：
    // - b（Box 的元数据：ptr, len, capacity）在栈上
    // - [0i32; 1000] 数据在堆上
} // b 离开作用域，Box::drop() 被调用
  // drop 内部调用分配器的 deallocate 释放堆内存
```

### 分配器的角色

Rust 默认使用 jemalloc（Linux/macOS）或系统分配器（Windows）。当 drop 被调用时：

```rust
impl Drop for Box<T> {
    fn drop(&mut self) {
        // jemalloc / libc::free / etc.
        // 将内存归还给操作系统或分配器缓存
    }
}
```

---

## 对比 Java/Go 的内存管理

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **内存释放时机** | 作用域结束时确定 | GC 运行时不确定 | GC 运行时不确定 |
| **回收算法** | 无（直接 drop） | 标记-清除 / G1 GC | 三色标记 + 混合 GC |
| **GC 暂停** | 无 | stop-the-world | stop-the-world（可优化） |
| **内存释放保证** | 确定性的 | 非确定性的 | 非确定性的 |
| **循环引用** | 需要 Weak/Arc | GC 自动处理 | GC 自动处理 |

### Java 的 GC 问题

```java
public class Main {
    public static void main(String[] args) {
        List<String> list = new ArrayList<>();
        for (int i = 0; i < 1000000; i++) {
            list.add(new String("data"));
        }
        list = null; // 列表不再被引用
        // 什么时候 GC 回收这块内存？不知道
        // 可能是 1ms 后，可能是 1 小时后
        // 这就是"非确定性析构"
    }
}
```

### Rust 的确定性析构

```rust
fn main() {
    let mut list = Vec::new();
    for i in 0..1000000 {
        list.push(String::from("data"));
    }
    // list 离开作用域，Vec::drop() 被立即调用
    // 内存被立即归还，不等待任何 GC
}
```

---

## 高级话题：循环引用与 Weak/Arc

### 为什么需要显式处理循环引用

```rust
// 这段代码有内存泄漏（在 Rust 2018 之前）
use std::cell::RefCell;
use std::rc::Rc;

struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
    prev: Option<Rc<RefCell<Node>>>, // 循环引用：Node -> Node
}

fn main() {
    let a = Rc::new(RefCell::new(Node {
        value: 1,
        next: None,
        prev: None,
    }));
    let b = Rc::new(RefCell::new(Node {
        value: 2,
        next: None,
        prev: None,
    }));

    // a.next = Some(b.clone());
    // b.prev = Some(a.clone());
    // 此时 a 和 b 的引用计数都是 2
    // 它们离开作用域时，Rc 计数不会降到 0
    // 因为互相持有引用，内存泄漏！
}
```

**Rust 的解法**：`Weak<T>` 不增加引用计数，用于打破循环：

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    value: i32,
    next: Option<Weak<RefCell<Node>>>, // Weak 不增加引用计数
    prev: Option<Weak<RefCell<Node>>>,
}

fn main() {
    let a = Rc::new(RefCell::new(Node {
        value: 1,
        next: None,
        prev: None,
    }));
    let b = Rc::new(RefCell::new(Node {
        value: 2,
        next: None,
        prev: None,
    }));

    // a.borrow_mut().next = Some(Rc::downgrade(&b));
    // b.borrow_mut().prev = Some(Rc::downgrade(&a));
    // 引用计数不会进入循环：
    // - a 和 b 的 strong_count 都是 1
    // Weak::upgrade() 返回 Option<Rc<T>>
}
```

---

## 设计哲学总结

### Rust 的内存管理是 RAII 的进化

RAII（Resource Acquisition Is Initialization）起源于 C++：
```cpp
class File {
    FILE* f;
public:
    File(const char* name) { f = fopen(name, "r"); }
    ~File() { fclose(f); } // 文件在析构时自动关闭
};
```

Rust 将 RAII 推广到**所有资源**，包括内存：

```rust
struct TCPConnection {
    socket: TcpStream,
}

impl Drop for TCPConnection {
    fn drop(&mut self) {
        // 连接在离开作用域时自动关闭
        // 这是确定性析构的真正威力
    }
}
```

### 为什么 Rust 不是"手动管理"

手动管理的定义：**程序员显式调用释放内存的代码**。

Rust 的不同：
1. 你不需要写 `free()` 或 `deallocate()`
2. 你不需要追踪"这个指针是否有效"
3. 编译器在编译时就确定了 drop 的位置

**Rust 是自动管理，但不是 GC**。区别在于"自动"的实现方式：
- GC：运行时追踪引用，推测式回收
- Rust：编译时确定 owner，确定性析构

---

## 常见误解澄清

### 误解 1：Rust 没有垃圾回收，所以是手动管理

**错误**。手动管理需要程序员显式调用释放代码。Rust 通过 Drop trait 自动释放，完全自动化。

### 误解 2：栈上的数据不需要回收

**正确但不完整**。栈上的数据确实在函数返回时自动释放（通过弹栈），但这不意味着"所有栈数据都不需要考虑"。Rust 的 Copy trait 就是用来标记"可以直接复制的栈上数据"。

### 误解 3：Rust 不可能有内存泄漏

**错误**。Rust 防止了 use-after-free 和 double-free，但循环引用（Rc/RefCell 的场景）仍然可能导致内存泄漏。Rust 通过 Weak 来解决这个问题，但需要程序员显式使用。

### 误解 4：Box<T> 和 Vec<T> 的回收方式不同

**错误**。它们都是通过 Drop trait 回收：
- `Box<T>`：调用 `allocator::deallocate`
- `Vec<T>`：调用 `Vec::drop` → `allocator::deallocate`
- `String`：调用 `String::drop` → `allocator::deallocate`

本质相同：都通过 Drop 契约释放堆内存。
