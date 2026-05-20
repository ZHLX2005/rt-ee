# Rust 如何检测内存泄漏

## 核心问题

Rust 声称"内存安全"，但**内存泄漏（Memory Leak）**在某些场景下仍然可能发生。Rust 是如何检测这些泄漏的？

---

## Rust 的内存安全保证

### Rust 防止的内存错误

| 错误类型 | Rust 处理方式 | 检测阶段 |
|---------|--------------|---------|
| use-after-free | 借用检查器 | 编译时 |
| double-free | 所有权 + Move | 编译时 |
| 悬空指针 | 生命周期检查 | 编译时 |
| 数据竞争 | Send/Sync 检查 | 编译时 |

### Rust 无法防止的泄漏

| 泄漏类型 | 原因 | 示例 |
|---------|------|------|
| 循环引用 | Rc/RefCell 互相持有 | `Rc::new(RefCell::new(Node { next: Some(rc.clone()) }))` |
| 故意不释放 | `mem::forget` | `std::mem::forget(x)` |
| 全局状态累积 | `Box::leak` | `Box::leak(Box::new(value))` |

---

## 编译时检测：借用检查器

### 借用检查器防止的是 use-after-free

```rust
fn main() {
    let r;
    {
        let x = 5;
        r = &x; // 错误！x 的生命周期不够长
    }
    println!("{}", r); // 使用已释放的内存
}
```

**编译器输出**：
```
error[E0597]: `x` does not live long enough
   --> src/main.rs:4:13
    |
4  |         r = &x;
    |             ^^ borrowed value does not live long enough
5  |     }
    |     - `x` dropped here while still borrowed
```

### 生命周期检查

```rust
fn dangle() -> &String { // 错误！
    let s = String::from("hello");
    &s // 返回对已释放内存的引用
}
```

**编译器输出**：
```
error[E0515]: cannot return reference to local variable `s`
    |
5  |     return &s;
    |            ^ returns a reference to a local variable
```

---

## 运行时检测：Miri

### 什么是 Miri？

Miri 是 Rust 的**解释器**，可以运行 Rust 代码并检测未定义行为（UB）。

```bash
# 安装 Miri
rustup component add miri

# 运行并检测内存错误
cargo miri run
```

### Miri 能检测的问题

```rust
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let x = Rc::new(RefCell::new(42));

    // 创建一个循环引用
    let x_clone = Rc::clone(&x);
    x.borrow_mut().push(Rc::downgrade(&x_clone)); // 模拟循环引用

    // Miri 会检测到 Rc 的引用计数没有归零
}
```

**Miri 输出**：
```
error: memory leak: Unreachable memory was never freed
```

### Miri 能检测的内存错误

| 错误类型 | 说明 |
|---------|------|
| 使用未初始化的内存 | `let x: i32; println!("{}", x);` |
| 越界访问 | `arr[100]` 超出数组范围 |
| 悬空指针解引用 | `&x` 后 x 被释放 |
| 双重释放 | 同一块内存 free 两次 |
| 内存泄漏 | Rc/Arc 循环引用 |

---

## 编译时检测：unsafe 代码

### Rust 对 unsafe 的约束

```rust
// Rust 的 unsafe 是受限的
unsafe fn dangerous() {
    // 只允许以下操作：
    // - 解引用原始指针
    // - 调用 unsafe 函数
    // - 访问可变静态变量
    // - 实现 unsafe trait
}
```

### -Z sanitizer：AddressSanitizer

```bash
# 编译时启用 ASan
RUSTFLAGS="-Z sanitizer=address" cargo run
```

AddressSanitizer 可以检测：
- 堆缓冲区溢出
- 栈缓冲区溢出
- 全局缓冲区溢出
- 释放后使用（use-after-free）
- 双重释放

---

## 循环引用的处理

### 问题：Rc<RefCell<Node>> 导致循环引用

```rust
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
    prev: Option<Rc<RefCell<Node>>>, // 循环引用！
}
```

### 解决方案：Weak<T>

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

    // 使用 Weak 打破循环
    let b_weak = Rc::downgrade(&b);
    a.borrow_mut().next = Some(b_weak);

    // b.prev 指向 a，但不会增加 a 的引用计数
    let a_weak = Rc::downgrade(&a);
    b.borrow_mut().prev = Some(a_weak);

    // 引用计数：a 和 b 都是 1
    // 没有内存泄漏！
}
```

### Weak vs Rc

| 特性 | Rc<T> | Weak<T> |
|------|--------|---------|
| 引用计数 | +1 | 不增加 |
| upgrade() | 返回 Rc | 返回 Option<Rc> |
| 用途 | 持有所有权 | 打破循环引用 |

---

## Box::leak 与 mem::forget

### Box::leak：故意泄漏

```rust
fn main() {
    let x = Box::new(42);
    let leak = Box::leak(x); // 永远不会被释放
    println!("{}", leak);    // 可以继续使用
}
```

**用途**：在全局状态中存储数据。

### mem::forget：跳过 Drop

```rust
use std::mem;

fn main() {
    let x = String::from("hello");
    mem::forget(x); // Drop 不会被调用
                    // 但内存不会被释放（泄漏）
}
```

**注意**：这是合法的，但不推荐。`ManuallyDrop` 可以更安全地达到同样效果。

---

## 检测工具总结

| 工具 | 类型 | 检测内容 |
|------|------|---------|
| 借用检查器 | 编译时 | use-after-free, 悬空指针 |
| 生命周期检查 | 编译时 | 生命周期不匹配 |
| Miri | 运行时/解释器 | UB, 内存泄漏, 未初始化内存 |
| AddressSanitizer | 运行时 | 堆溢出, use-after-free |
| Valgrind | 运行时 | 内存泄漏检测（通过 shadow memory） |

---

## 设计哲学

### Rust 的检测策略

1. **编译时优先**：借用检查器在编译时消除大多数内存错误
2. **运行时补充**：Miri 和 sanitizer 检测更复杂的问题
3. **显式优于隐式**：unsafe 代码需要程序员负责

### 为什么 Rust 不能检测所有泄漏？

- **循环引用有时是合理的**：图结构、树结构天然有父子双向引用
- **Weak 是显式选择**：Rust 不自动打断循环，因为有时循环是业务逻辑的一部分
- **性能考量**：运行时追踪所有引用会引入巨大开销

---

## 最佳实践

1. **优先使用所有权类型**：Box, Vec, String 等会自动 drop
2. **需要共享时用 Arc**：原子引用计数，没有循环引用问题
3. **打破循环用 Weak**：Rc + RefCell 时，用 Weak 替代部分 Rc
4. **避免 Box::leak**：除非有充分理由
5. **用 Miri 测试 unsafe**：定期运行 `cargo miri test`

---

## 参考

- `docs/memory_management/memory_management.md` — 内存管理机制详解
- `docs/ownership/ownership.md` — 所有权系统
