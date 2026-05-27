# 所有权 (Ownership)

## 设计背景与问题域

### 传统语言的困境

| 语言 | 内存管理方式 | 本质问题 |
|------|-------------|---------|
| C/C++ | 手动管理 | 悬挂指针、双重释放 |
| Java/Go | GC / 引用计数 | 非确定性析构、性能开销 |
| Rust | 所有权系统 | 编译时静态分析，零运行时开销 |

**Rust 试图回答一个问题**：能否在编译时就能确定每个值的生命周期，从而完全避免运行时开销？

### 为什么 Rust 选择"所有权"而非 GC

GC 的代价：
- 停顿（stop-the-world）
- 内存碎片化
- 非确定性（你不知道对象何时被释放）

所有权系统的代价：
- 学习曲线陡峭
- 某些编程模式无法表达

**Rust 的赌注**：用学习曲线换取确定性和性能。

---

## 抽象设计分析

### 所有权是一种线性类型系统

线性类型的核心规则：**每个值有且只有一个 owner，且只能被使用一次后转移**。

这与资源管理的对应关系：
- 内存 → 分配/释放
- 文件描述符 → 打开/关闭
- 锁 → 获得/释放

**设计启发**：将资源管理泛化为一种类型系统的规则，是 Rust 的核心创新。

### 与 Java/Go 的本质区别

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 内存分配 | 栈或堆，语义一致 | 堆为主 | 栈/堆，语义不透明 |
| 释放时机 | 作用域结束时确定 | GC 非确定 | GC 非确定 |
| 并发安全 | 编译时检查 | 运行时检查 | 运行时检查 |
| 空指针 | Option 类型 | NPE 运行时 | nil 指针 |

**关键洞察**：Rust 将"不可能状态"变为"不可能编译"，而 Java/Go 将其留到运行时。

---

## 核心规则

### 所有权三条规则

1. **每个值有一个 owner** — 变量持有值的所有权
2. **同一时间只有一个 owner** — 赋值或传参时发生 `move`（移动）
3. **当 owner 离开作用域，值被 drop** — 自动调用 drop 释放资源

### Move 语义的设计意图

```rust
let s1 = String::from("hello");
let s2 = s1; // s1 被移动到 s2
```

**为什么要让 s1 无效？**

- **确定性**：编译器能精确知道 s2 在何时负责释放
- **无双重释放**：不会有两个 owner 尝试释放同一块内存
- **零开销**：不需要引用计数（Rc/Arc 有额外开销）

### Copy trait：特殊情况的处理

对于可以在栈上"简单拷贝"的类型（i32、bool、char 等），Rust 自动实现 Copy trait：

```rust
let x = 5;
let y = x; // x 仍然有效，因为 i32 实现了 Copy
```

**设计决策**：Copy 类型不需要析构函数，所以直接拷贝是安全的。

---

## 代码示例（带设计意图注释）

### 示例 1：Move 语义

```rust
fn main() {
    // 设计意图：展示所有权转移
    // 为什么 s1 会无效？因为 String 没有 Copy trait
    // 对比：如果用 Java，s1 和 s2 都指向同一个对象（引用语义）

    let s1 = String::from("hello");
    let s2 = s1; // s1 的所有权转移给 s2，s1 不再有效

    // 如果 uncomment 下面这行，Java 会编译通过（两个引用指向同一对象）
    // Java: 内存泄漏或双重释放的风险被推迟到 GC
    // Rust: 编译时就杜绝了这种风险

    // println!("{}", s1); // 编译错误！s1 已无效

    println!("{}", s2); // s2 是唯一有效的 owner
} // s2 离开作用域，String 被 drop，内存被释放
```

### 示例 2：Clone vs Move

```rust
fn main() {
    let s1 = String::from("hello");

    // 设计意图：显式克隆 vs 隐式移动
    // 为什么要 .clone() 必须是显式的？
    // - 提醒开发者：这可能是有代价的操作
    // - 对比 Java：所有对象都是引用，隐式克隆可能是性能陷阱

    let s2 = s1.clone(); // 显式深拷贝，s1 仍然有效

    println!("s1 = {}, s2 = {}", s1, s2);
}
```

### 示例 3：函数参数的所有权

```rust
// 设计意图：所有权可以被函数"消费"
fn consume(s: String) {
    println!("consumed: {}", s);
} // s 离开作用域，String 被 drop

fn main() {
    let s = String::from("hello");
    consume(s); // s 的所有权转移给 consume
    // println!("{}", s); // 编译错误！s 已无效
}
```

**对比 Java**：Java 永远不会"消费"一个引用，引用始终有效（除非对象被 GC）。

### 示例 4：借用（Borrowing）

```rust
// 设计意图：借用允许使用值但不获取所有权
// 为什么需要借用？因为有时候你只是"看"而不需要"拥有"
fn calculate_length(s: &String) -> usize {
    s.len()
} // s 离开作用域，但不 drop 其指向的值

fn main() {
    let s1 = String::from("hello");

    // 设计意图：借用允许函数使用值而不获取所有权
    // 对比 Java：所有方法都是隐式借用（除了基本类型是拷贝）
    // 对比 Go：指针语义，但没有 Rust 的借用检查规则

    let len = calculate_length(&s1); // & 表示借用

    println!("'{}' 的长度是 {}", s1, len); // s1 仍然有效
} // s1 被 drop
```

### 示例 5：借用检查器的引导

```rust
fn main() {
    let mut s = String::from("hello");

    // 设计意图：借用检查器如何引导你写出正确代码
    // 编译器的错误信息不是嘲讽，而是告诉你如何修复

    let r1 = &s; // 第一个不可变借用
    let r2 = &s; // 第二个不可变借用 OK（同时多个不可变借用是可以的）

    println!("{} and {}", r1, r2);
    // r1 和 r2 在这里之后不再使用

    let r3 = &mut s; // 可变借用 OK，因为 r1, r2 已不再使用
    println!("{}", r3);
}
```

**关键设计**：借用检查器使用"非词法作用域生命周期"（NLL），只在最后一次使用之后才算借用结束。这比早期的作用域规则更灵活。

---

## 进阶：Move 的底层机制

### Move 之后，内存到底发生了什么？

这是从 Java/Go 转来的程序员最困惑的问题之一。

```rust
let s1 = String::from("hello"); // s1 在栈上，包含[指针, 长度, 容量]
let s2 = s1;                    // move 发生
```

**在汇编层面**，`let s2 = s1` 只做了一件事：**把 s1 在栈上的 24 个字节按位拷贝到 s2 的位置**。没有魔法，没有运行时检查，就是一次 `mov` 指令。

那"s1 无效"是什么意思？**这是编译器层面的状态追踪，不是运行时的物理清零**。

```
栈帧布局（64位）：

move 前：
  s1: [ptr=0x7f...1000, len=5, cap=5]  ← 指向堆上的 "hello"

move 后（物理内存）：
  s1: [ptr=0x7f...1000, len=5, cap=5]  ← 原值还在！只是编译器禁止你读
  s2: [ptr=0x7f...1000, len=5, cap=5]  ← 按位拷贝过来的副本

编译器视角：
  s1: [MOVED — 未初始化]              ← 状态标记
  s2: [Initialized — 拥有所有权]       ← 唯一合法的 owner
```

**为什么不会"堆栈混乱"？**

1. **物理层面没有混乱**：s1 的内存只是原样保留（为了性能，不会清零），栈帧结构完全正常
2. **逻辑层面被严格管控**：编译器在 MIR 上追踪每个变量的初始化状态，`s1` 被标记为 `Moved`
3. **Drop 只执行一次**：编译器保证只有 `s2` 的析构函数会被调用，`s1` 的析构被跳过

对比 Java：
```java
String s1 = new String("hello");
String s2 = s1;  // s1 和 s2 都引用同一个对象，对象头引用计数+1（或 GC 追踪）
// s1 仍然完全可用
```
Java 需要运行时机制（GC 或引用计数）来管理共享所有权。Rust 在编译期就消除了"共享"的可能性。

### 编译器如何在编译期检测 use-after-move？

Rust 编译器在 **MIR（Mid-level IR）** 阶段执行**Move Path Analysis**（移动路径分析）。

```rust
fn main() {
    let s1 = String::from("hello");  // s1: Initialized
    let s2 = s1;                     // s1: Moved → s2: Initialized
    println!("{}", s1);              // 编译错误！s1 状态为 Moved
}
```

编译器为每个变量路径维护一个状态机：

| 状态 | 含义 | 对 use 的反应 |
|------|------|-------------|
| `Uninitialized` | 变量声明但尚未赋值 | 禁止使用 |
| `Initialized` | 变量持有有效值 | 允许使用 |
| `Moved` | 值已被转移给其他变量 | **禁止使用，报错** |
| `PartiallyMoved` | 复合类型的部分字段被移动 | 整体和部分都受限制 |

这个分析在**控制流图（CFG）**上进行：

```
MIR 控制流图（简化）：

bb0: {
    _1 = String::from("hello");     // _1 状态: Initialized
    _2 = move _1;                    // _1 状态: Moved, _2 状态: Initialized
    _3 = _1;                         // 错误！_1 在 bb0 入口时是 Moved
    return;
}
```

编译器遍历每个基本块（basic block），追踪每个路径的状态。当发现对已 `Moved` 的路径进行读取时，立即报错：

```
error[E0382]: borrow of moved value: `s1`
  --> src/main.rs:4:14
   |
 2 |     let s1 = String::from("hello");
   |         -- move occurs because `s1` has type `String`,
   |            which does not implement the `Copy` trait
 3 |     let s2 = s1;
   |              -- value moved here
 4 |     println!("{}", s1);
   |                    ^^ value borrowed here after move
```

### 为什么能做到编译期检测？

**关键洞察**：Rust 的 move 不是运行时的操作，而是**编译期的状态转换**。

```
Java 的引用语义：
  运行时：两个引用指向同一对象 → 需要 GC/引用计数决定何时释放
  问题：编译器无法知道一个引用是否仍然有效（因为可能有第三个引用）

Rust 的所有权语义：
  编译期：值从 A 转移到 B，A 永久失效 → 编译器精确知道唯一 owner
  结果：不需要运行时追踪，状态分析在编译期完成
```

Rust 能做到这一点的前提是**所有权规则的严格性**：
- 每个值只能有一个 owner
- 没有隐式共享（必须通过 `Rc`/`Arc` 显式选择）
- 没有隐式拷贝（必须通过 `.clone()` 显式选择）

这种严格性使得编译器可以在 MIR 上静态分析所有可能的执行路径，而不需要运行时信息。

### Partial Move：结构体字段的移动

Move 分析不仅作用于整个变量，还精确到字段级别：

```rust
struct Person {
    name: String,
    age: u32,
}

fn main() {
    let p = Person {
        name: String::from("Alice"),
        age: 30,
    };

    let name = p.name;  // p.name 被 move
    // println!("{:?}", p);      // 编译错误！p 部分移动
    // println!("{}", p.name);   // 编译错误！p.name 已移动
    println!("{}", p.age);       // OK，p.age 没有被移动
}
```

编译器对 `p` 的每个字段分别追踪状态：`p.name` 是 `Moved`，`p.age` 是 `Initialized`。这种细粒度的追踪确保了：
- 不会双重释放 `p.name`
- 不会读取未初始化的 `p.name`
- 但仍然允许安全地使用 `p.age`

---

## 设计哲学

### 所有权系统的代价与收益

**代价**：
- 学习曲线：需要理解 move、borrow、lifetime
- 某些模式无法表达：循环引用（需要 Rc/Arc）

**收益**：
- 确定性析构：资源在离开作用域时立即释放
- 零运行时开销：不需要 GC 或引用计数
- 数据竞争不可能：编译时检查并发安全
- 消除整个类别的 bug：空指针、悬挂指针、双重释放

### 对 Java/Go 程序员的启发

**Java 程序员应该思考**：
- 为什么 Java 选择 GC 而不是所有权系统？
- NPE 是个设计缺陷还是不可避免的代价？
- Java 的"一切皆对象" vs Rust 的"一切皆值"，对性能有什么影响？

**Go 程序员应该思考**：
- Go 的 nil 指针和 Rust 的 Option 类型，本质区别是什么？
- Go 的 GC 停顿是否可接受？Rust 的所有权系统是否值得学习曲线？
- 为什么 Go 的 channel 被设计为值类型传递（实际上也是移动语义）？

---

## 运行

```bash
cargo run -p ownership
```
