# MIR 与借用检查器算法深度解析

## 设计背景与问题域

Rust 的内存安全不是由运行时 GC 保证的，而是由编译器的**借用检查器（Borrow Checker）**在编译期证明的。借用检查器运行在一种特殊的中间表示——**MIR（Mid-level IR）**——之上。

理解 MIR 和借用检查器需要回答：

1. **MIR 是什么？** 为什么不用 AST 或 HIR 直接做借用检查？
2. **借用检查器的输入和输出是什么？** 它如何"证明"程序是安全的？
3. **NLL（Non-Lexical Lifetimes）** 解决了什么问题？
4. **区域推断（Region Inference）** 如何将生命周期标注转化为可验证的约束？

---

## MIR：面向分析的中间表示

### 为什么需要 MIR？

AST 和 HIR 是**树形结构**，适合语法分析和类型检查，但不适合**数据流分析**。借用检查需要回答："在程序的第 N 行，变量 X 是否仍然有效？"——这是一个关于**控制流**的问题。

MIR 将函数表示为**控制流图（Control Flow Graph, CFG）**：
- **节点**：基本块（BasicBlock）
- **边**：跳转关系（goto、if、return、panic）

### MIR 的核心数据结构

```
函数体（Body）
├── local_decls: Vec<LocalDecl>  // 局部变量声明（包括参数、返回值、临时变量）
├── basic_blocks: Vec<BasicBlockData>  // 基本块列表
│   └── BasicBlockData
│       ├── statements: Vec<Statement>  // 语句序列（顺序执行）
│       └── terminator: Terminator  // 终止指令（跳转、返回、panic）
└── arg_count: usize  // 参数数量
```

### MIR 的基本元素

#### Local（局部变量）

MIR 中所有值都存储在 **local** 中，用 `_0`, `_1`, `_2`... 编号：

- `_0`：函数的返回值（始终存在）
- `_1` ~ `_N`：函数参数
- `_N+1` ~ `_M`：用户声明的局部变量和编译器生成的临时变量

```rust
fn foo(x: i32, y: String) -> i32 {
    let z = x + 1;
    z
}

// MIR local 编号：
// _0: i32（返回值）
// _1: i32（参数 x）
// _2: String（参数 y）
// _3: i32（局部变量 z）
// _4: i32（临时变量：x + 1 的结果）
```

#### Place（位置）和 Rvalue（右值）

MIR 将赋值语句拆分为 **Place = Rvalue**：

- **Place**：值的存储位置（局部变量、字段、数组索引）
- **Rvalue**：产生新值的表达式

```rust
let x = a + b;        // Place = _3, Rvalue = Add(_1, _2)
let y = *ref_x;       // Place = _4, Rvalue = Deref(_5)
let z = move(s);      // Place = _6, Rvalue = Move(_7)
```

#### Statement（语句）

基本块内的语句按顺序执行，不会跳转：

```
Statement::Assign(Place, Rvalue)     // 赋值
Statement::StorageLive(Local)        // 标记变量开始存活
Statement::StorageDead(Local)        // 标记变量结束存活
Statement::SetDiscriminant(...)      // 设置枚举变体标签
Statement::FakeRead(...)             // 为借用检查生成的假读
```

#### Terminator（终止指令）

每个基本块的结尾是一个终止指令，决定控制流的去向：

```
Terminator::Goto { target }                    // 无条件跳转
Terminator::SwitchInt { discr, targets }       // 整数匹配跳转
Terminator::Return                             // 返回
Terminator::Call { func, args, destination, cleanup }  // 函数调用
Terminator::Assert { cond, expected, msg, target, cleanup }  // 断言
Terminator::Drop { place, target, unwind }     // 调用析构函数
Terminator::Resume                             // 继续 panic 展开
```

### 一个完整的 MIR 示例

```rust
fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}
```

生成的 MIR（简化）：

```
fn max(_1: i32, _2: i32) -> i32 {
    let mut _0: i32;
    let mut _3: bool;

    bb0: {
        _3 = Gt(move _1, move _2);      // _3 = a > b
        switchInt(move _3) -> [false: bb2, otherwise: bb1];
    }

    bb1: {                              // a > b 为真
        _0 = move _1;                   // 返回 a
        goto -> bb3;
    }

    bb2: {                              // a > b 为假
        _0 = move _2;                   // 返回 b
        goto -> bb3;
    }

    bb3: {                              // 汇合点
        return;
    }
}
```

---

## 借用检查器：基于 MIR 的数据流分析

### 借用检查器的输入和输出

**输入**：
- MIR（控制流图）
- 类型信息（哪些类型实现了 Copy、哪些需要 Drop）
- 用户标注的生命周期（`'a`, `'b`）

**输出**：
- 编译错误（如果检测到违反借用规则）
- 或者"证明通过"（程序被证明内存安全）

### 核心数据结构：BorrowSet

借用检查器为每个基本块的每个程序点维护一组**事实（facts）**：

```
在程序点 P：
- 哪些路径被借用了（共享引用 &T 或可变引用 &mut T）
- 哪些值的所有权已被转移（moved）
- 哪些值已被丢弃（dropped）
```

### 核心规则（在 MIR 上的表达）

借用检查器在 MIR 上验证以下规则：

#### 规则 1：不可变借用（Shared Borrow）

```rust
let x = String::from("hello");
let r1 = &x;       // 共享借用 x
let r2 = &x;       // OK：可以同时存在多个共享借用
println!("{}", r1);
```

MIR 级别的约束：
- `&x` 创建了一个共享借用，在 `r1` 和 `r2` 的生命周期内，`x` 不能被可变借用，也不能被 move。

#### 规则 2：可变借用（Mutable Borrow）

```rust
let mut x = String::from("hello");
let r1 = &mut x;   // 可变借用 x
// let r2 = &mut x; // ERROR：不能同时存在两个可变借用
*r1 += " world";
```

MIR 级别的约束：
- `&mut x` 创建了一个可变借用，在其生命周期内，`x` 不能被任何其他借用（共享或可变）访问。

#### 规则 3：所有权转移（Move）

```rust
let s1 = String::from("hello");
let s2 = s1;       // s1 的所有权转移给 s2
// println!("{}", s1); // ERROR：s1 已被 move
```

MIR 级别的约束：
- `move s1` 将 `s1` 标记为"已移动"。后续任何对 `s1` 的使用都是编译错误。

---

## NLL：非词法生命周期

### 问题：词法作用域的局限性

在 Rust 2015 edition 中，引用的生命周期绑定到**词法作用域**（花括号），导致很多安全的代码无法编译：

```rust
fn main() {
    let mut x = String::from("hello");
    let y = &x;           // y 的生命周期 = 整个 main 函数
    println!("{}", y);
    // y 在这里已经不用了，但词法作用域认为它仍然存活
    let z = &mut x;       // 2015 edition：ERROR！y 仍然"存活"
    println!("{}", z);
}
```

### NLL 的解决方案：基于数据流的生命周期

Rust 2018 引入 **NLL（Non-Lexical Lifetimes）**，引用的生命周期不再绑定到词法作用域，而是绑定到**最后一次使用的位置**。

```rust
fn main() {
    let mut x = String::from("hello");
    let y = &x;           // y 创建
    println!("{}", y);    // y 最后一次使用
    // ← NLL 认为 y 在这里结束
    let z = &mut x;       // OK！y 已经"死亡"
    println!("{}", z);
}
```

### NLL 算法原理

NLL 算法在 MIR 上执行**数据流分析**：

1. **构建借用图**：记录每个借用（`&x`、`&mut x`）的创建点和所有使用点
2. **计算活跃集（Liveness）**：对于每个程序点，计算哪些借用仍然"活跃"（之后还会被使用）
3. **冲突检测**：检查是否存在两个冲突的借用同时活跃

```
活跃性分析（Liveness Analysis）：

对于每个基本块，从后向前计算：
- 如果变量 V 在当前语句被使用，则 V 在进入当前语句时是活跃的
- 如果变量 V 在当前语句被赋值，则 V 在进入当前语句时不活跃

借用检查：
对于每个程序点 P：
- 如果存在一个可变借用 &mut x 活跃
- 则不允许任何其他借用（&x 或 &mut x）活跃
- 也不允许对 x 进行 move
```

---

## 区域推断（Region Inference）

### 生命周期是约束系统

用户写的 `'a` 不是运行时的东西，而是编译期的**约束变量**。借用检查器的任务是将这些约束转化为可求解的方程组。

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}
```

约束系统：
- 返回值的生命周期 `'a` 必须不超过 `x` 的生命周期
- 返回值的生命周期 `'a` 必须不超过 `y` 的生命周期
- 即：`'a ⊆ lifetime(x)` 且 `'a ⊆ lifetime(y)`

### 区域推断算法

1. **为每个引用类型创建区域变量**（每个 `&T` 和 `&mut T` 都有一个隐式的生命周期变量）
2. **从 MIR 中提取约束**：
   - 赋值约束：`let r = &x` → `lifetime(r) ⊆ lifetime(x)`
   - 返回约束：`return r` → `lifetime(r) ⊆ lifetime(return_value)`
   - 函数调用约束：`foo(&x)` → 根据函数签名施加约束
3. **求解约束系统**：使用**最小不动点算法**找到满足所有约束的最小生命周期
4. **验证**：检查是否存在垂悬引用（即某个引用的生命周期超出了被引用数据的生命周期）

### 与类型推断的类比

生命周期推断和类型推断（Hindley-Milner）在数学上是同构的：

| | 类型推断 | 生命周期推断 |
|---|---|---|
| 变量 | 类型变量 `?T` | 区域变量 `'?a` |
| 约束 | 统一约束 `?T = i32` | 子集约束 `'?a ⊆ '?b` |
| 求解 | 最小泛化（most general unifier） | 最小不动点 |

---

## 两点借用（Two-Phase Borrows）

### 问题场景

```rust
let mut vec = vec![1, 2, 3];
vec.push(vec.len());  // 这里发生了什么？
```

`vec.push(...)` 需要 `&mut vec`，而 `vec.len()` 需要 `&vec`。按照传统借用规则，这是冲突的。

### 两点借用的解决方案

Rust 编译器将 `&mut vec` 的借用分为两个阶段：

1. **保留阶段（Reservation Phase）**：在计算参数之前，创建一个"保留"的可变借用。此时不允许其他可变借用，但允许共享借用。
2. **激活阶段（Activation Phase）**：在参数计算完成后，将保留升级为真正的可变借用。

```
vec.push(vec.len())

Phase 1（保留）：&mut vec 被保留
    ├── vec.len() 计算：&vec 被创建（OK，保留阶段允许共享借用）
    └── vec.len() 返回
Phase 2（激活）：&mut vec 被激活
    ├── push 执行
    └── &mut vec 释放
```

这是 Rust 编译器为了支持常见模式而做的**特殊例外**，但只在方法调用的 `self` 参数上生效。

---

## 实际案例：代码 → MIR → 借用检查

### 案例 1：简单的所有权转移

```rust
fn main() {
    let s = String::from("hello");
    let t = s;           // move
    println!("{}", s);   // ERROR
}
```

MIR（简化）：

```
fn main() -> () {
    let _1: String;      // s
    let _2: String;      // t

    bb0: {
        _1 = String::from(const "hello");
        _2 = move _1;    // _1 被标记为 moved
        // println!("{}", _1);  // 编译错误！_1 已被 move
        return;
    }
}
```

借用检查器在 `move _1` 之后将 `_1` 加入 "moved set"。后续任何对 `_1` 的使用都会触发错误。

### 案例 2：共享借用与可变借用的冲突

```rust
fn main() {
    let mut x = String::from("hello");
    let r1 = &x;
    let r2 = &mut x;     // ERROR
    println!("{}", r1);
}
```

MIR（简化）：

```
fn main() -> () {
    let mut _1: String;  // x
    let _2: &String;     // r1
    let _3: &mut String; // r2

    bb0: {
        _1 = String::from(const "hello");
        _2 = &_1;        // 共享借用 _1
        _3 = &mut _1;    // ERROR！_1 已被共享借用，不能可变借用
        // ...
    }
}
```

借用检查器在 `_3 = &mut _1` 处检测到冲突：`_1` 已经存在一个活跃的共享借用 `_2`。

### 案例 3：NLL 允许的安全代码

```rust
fn main() {
    let mut x = String::from("hello");
    let r = &x;
    println!("{}", r);   // r 最后一次使用
    // ← NLL：r 在这里结束
    let m = &mut x;      // OK！r 已经不活跃了
    m.push_str(" world");
}
```

借用检查器的数据流分析：
- `r` 在 `println!` 之后不再活跃
- `m = &mut x` 处，`r` 已不在活跃集中
- 因此 `&mut x` 是合法的

---

## 与 Java/Go 的根本差异

| 维度 | Rust（MIR + 借用检查器） | Java（JVM） | Go（Go runtime） |
|------|------------------------|------------|-----------------|
| 内存安全检查 | 编译期数据流分析 | 运行时 GC + NullPointerException | 运行时 GC + panic |
| 分析表示 | MIR（控制流图） | 无（运行时对象图） | 无（运行时对象图） |
| 垂悬引用检测 | 编译期拒绝 | 运行时报错（NPE） | 运行时报错（panic） |
| 数据竞争检测 | 编译期拒绝（Send/Sync） | 无检测（依赖 synchronized） | 无检测（依赖 channel） |
| 性能成本 | **零** | GC 暂停 + 边界检查 | GC 暂停 |

---

## 总结

MIR 和借用检查器的设计体现了 Rust 的核心工程哲学：**将运行时的复杂性搬到编译期**。

| 组件 | 职责 | 创新点 |
|------|------|--------|
| MIR | 控制流图表示 | 将 Rust 语义降维到基本块和赋值语句 |
| 借用检查器 | 数据流分析 | 在编译期证明内存安全 |
| NLL | 非词法生命周期 | 用活跃性分析替代词法作用域 |
| 区域推断 | 约束求解 | 将生命周期标注转化为可求解的方程组 |

这些算法共同支撑了 Rust "零成本抽象 + 内存安全"的承诺——编译器承担了所有验证工作，运行时只剩下高效的机器码。

---

## 延伸阅读

- [Rust 编译器管线深度解析](compiler_pipeline.md)

## 查看 MIR 的方法

```bash
# 查看未优化的 MIR
rustc +nightly -Z mir-opt-level=0 --emit=mir src/main.rs

# 查看优化后的 MIR
rustc +nightly --emit=mir src/main.rs

# 查看特定函数的 MIR（cargo）
cargo rustc -- --emit=mir -Z mir-opt-level=0

# 查看借用检查器的诊断信息
rustc +nightly -Z borrowck=mir -Z verbose
```
