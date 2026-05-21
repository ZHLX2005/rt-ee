# Rust 编译器管线深度解析：为什么 rustc 有这么多阶段？

## 设计背景与问题域

用户观察到 Rust 编译器"任务多"、"设计牛逼"。这不是错觉——rustc 确实是现代语言中最复杂的编译器之一。要理解为什么，需要回答：

1. **为什么 rustc 需要 5+ 种中间表示（IR）？** Java 的 javac 只有 AST → bytecode，Go 的编译器只有 AST → SSA → machine code。
2. **rustc 在每个阶段做了什么其他语言不做的事情？**
3. **这种复杂度换来了什么？** 编译时间 vs 运行时性能 vs 安全保证的权衡。

---

## rustc 编译管线全景

```
源代码 (.rs)
    ↓
Lexer → Token Stream（词法单元流）
    ↓
Parser → AST（抽象语法树）
    ↓
Macro Expansion（过程宏/声明式宏展开）
    ↓
Name Resolution（名称解析）
    ↓
HIR（High-level IR）→ 面向编译器的高级中间表示
    ↓
Type Checking（类型检查，trait 解析）
    ↓
THIR（Typed HIR）→ 类型化的 HIR
    ↓
MIR（Mid-level IR）→ **Rust 独有的核心中间表示**
    ↓
Borrow Checking（借用检查）
    ↓
Optimization（常量传播、死代码消除等）
    ↓
LLVM IR → 机器无关的低级中间表示
    ↓
LLVM Optimization（内联、循环优化、向量化等）
    ↓
Machine Code（目标机器码）
    ↓
Linking（链接）
    ↓
可执行文件
```

---

## 每个阶段的职责与设计意图

### 1. Lexer + Parser → AST

**职责**：将文本转换为结构化的语法树。

```rust
// 源代码
let x = 42 + 1;

// AST（简化表示）
Stmt::Let {
    pat: Pat::Ident("x"),
    init: Expr::Binary {
        op: BinOp::Add,
        left: Expr::Lit(42),
        right: Expr::Lit(1),
    }
}
```

**与 Java/Go 对比**：
- Java：javac 也有 AST，但直接生成 bytecode，没有后续的高级 IR
- Go：go/parser 生成 AST，直接进入类型检查和 SSA

### 2. HIR（High-level IR）

**职责**：去掉语法糖，保留 Rust 的核心语义。

```rust
// 源代码中的 if let
if let Some(x) = opt { ... }

// HIR 展开为 match
match opt {
    Some(x) => { ... },
    _ => {},
}
```

**为什么需要 HIR？**
- AST 直接映射语法，包含太多语法糖（`if let`、`for` 循环、`?` 运算符）
- HIR 是**面向编译器**的表示，每个构造都有明确的语义
- 简化后续类型检查和分析

**Java/Go 对比**：
- Java 没有 HIR，AST 直接用于生成 bytecode
- Go 也没有类似 HIR 的阶段（但 SSA 承担了部分简化工作）

### 3. Type Checking + Trait Resolution

**职责**：验证类型正确性，解析 trait bounds，选择具体实现。

```rust
fn max<T: PartialOrd>(a: T, b: T) -> T {
    if a > b { a } else { b }
}

// 类型检查器验证：
// 1. T 满足 PartialOrd bound
// 2. `a > b` 对 PartialOrd 类型合法
// 3. 两个分支返回相同类型 T
```

**关键复杂性**：Rust 的类型系统支持**关联类型、高阶 trait bounds、生命周期参数化**，类型检查远比 Java/Go 复杂。

### 4. THIR（Typed HIR）

**职责**：HIR 经过类型检查后的产物，所有表达式都标注了具体类型。

THIR 相对较新（Rust 1.45+），目的是：
- 将类型信息注入 IR，为 MIR 生成做准备
- 处理模式匹配的穷尽性检查
- 安全检查（如整数溢出检查、`unsafe` 块验证）

### 5. MIR（Mid-level IR）— Rust 独有的核心

**职责**：控制流图（CFG）表示，是借用检查、数据流分析、优化的基础。

```rust
// 源代码
fn swap(a: &mut i32, b: &mut i32) {
    let temp = *a;
    *a = *b;
    *b = temp;
}

// MIR（简化）
fn swap(_1: &mut i32, _2: &mut i32) -> () {
    let mut _0: ();
    let _3: i32;

    bb0: {
        _3 = (*_1);            // temp = *a
        (*_1) = (*_2);         // *a = *b
        (*_2) = _3;            // *b = temp
        return;
    }
}
```

**为什么需要 MIR？**

| 语言 | 内存安全检查机制 | 实现位置 |
|------|---------------|---------|
| Java | GC | 运行时 JVM |
| Go | GC | 运行时 Go runtime |
| C/C++ | 手动管理 | 程序员 responsibility |
| **Rust** | **借用检查器** | **编译期（基于 MIR）** |

**核心洞察**：Rust 编译器在 MIR 阶段执行了其他语言在**运行时**做的内存安全检查。

- Java：运行时有 GC 线程扫描堆内存
- Go：运行时有 GC 暂停程序清理内存
- **Rust**：编译时分析 MIR 的控制流图，证明程序不存在数据竞争和垂悬引用

这正是 rustc "任务多"的根本原因——**它在编译期做了其他语言的运行时系统做的事情**。

### 6. Borrow Checking（借用检查）

**职责**：在 MIR 上执行数据流分析，验证所有权和借用规则。

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;          // s1 的所有权转移给 s2
    println!("{}", s1);   // 编译错误！借用检查器在 MIR 上发现 s1 已失效
}
```

借用检查器在 MIR 上追踪每个值的**生命周期（lifetime）**：
- 值何时创建（赋值）
- 值何时转移（move）
- 值何时借用（共享引用 `&` 或可变引用 `&mut`）
- 值何时销毁（drop）

这是 Rust 的**核心创新**，也是编译时间的主要来源之一。

### 7. LLVM IR → Machine Code

rustc 不直接生成机器码，而是将 MIR 翻译为 **LLVM IR**，复用 LLVM 的成熟优化和后端：

```
MIR
    ↓ rustc_codegen_llvm
LLVM IR
    ↓ LLVM Passes
Optimized LLVM IR
    ↓ LLVM CodeGen
Assembly / Machine Code
```

**为什么用 LLVM？**
- LLVM 支持数十种目标架构（x86、ARM、RISC-V、WASM）
- LLVM 的优化 Pass 极其成熟（内联、循环优化、向量化、LTO）
- Rust 团队不需要维护独立的后端

**Go 的不同选择**：Go 使用自己的后端（从 Go 1.5 开始不再依赖 C 编译器），追求更快的编译速度。

---

## 查询系统（Query System）：增量编译的基石

rustc 使用**基于查询的编译模型**，这是现代编译器设计的先进范式：

```
编译器不是"阶段式"地处理整个 crate，而是按需执行查询：

"这个函数的类型是什么？" → 触发类型检查查询
"这个函数是否安全？" → 触发借用检查查询
"这个表达式是否可以常量求值？" → 触发常量评估查询
```

**查询系统的优势**：
1. **按需计算**：只编译被使用的代码（dead code elimination 天然支持）
2. **缓存友好**：每个查询的结果被缓存，未变更的代码直接复用
3. **并行化**：独立的查询可以并行执行
4. **增量编译**：通过追踪查询依赖图，只重新计算受影响的查询

**与 Java/Go 对比**：
- Java：javac 是传统的阶段式编译器，没有查询系统（但 Eclipse/IntelliJ 有自己的增量编译模型）
- Go：go build 支持增量编译，但基于文件时间戳和包粒度，粒度较粗
- Rust：查询系统基于**细粒度的语义单元**（函数、类型、trait impl），粒度更细

---

## 为什么 rustc 编译"慢"？

### 编译时间的来源

| 阶段 | 时间占比 | 说明 |
|------|---------|------|
| 解析 + 宏展开 | ~5% | 相对快速 |
| 类型检查 + trait 解析 | ~20% | Rust 类型系统复杂 |
| MIR 生成 + 借用检查 | ~25% | **Rust 独有的开销** |
| 代码生成（LLVM） | ~40% | LLVM 优化 Pass 耗时 |
| 链接 | ~10% | 大型项目可能成为瓶颈 |

### 与其他语言的对比

| 语言 | 编译速度 | 原因 |
|------|---------|------|
| Go | 极快 | 类型系统简单，优化较少，自研后端 |
| Java | 快 | javac 只做语法和类型检查，优化交给 JIT |
| C++ | 慢 | 模板实例化、头文件包含、复杂优化 |
| **Rust** | **较慢** | **类型系统复杂 + 借用检查 + LLVM 优化** |

**关键洞察**：Rust 的编译时间是一种**预付费**——编译期做的所有检查（借用、生命周期、trait 解析）都转化为运行时的零成本。

- Java：编译快，但运行时 JVM 需要加载类、JIT 编译、GC 扫描
- Go：编译快，但运行时有 GC 开销
- **Rust**：编译慢，但运行时只有纯机器码，无任何运行时系统开销

---

## 实用工具：窥探编译器内部

### 查看 AST

```bash
rustc +nightly -Z ast-json src/main.rs
```

### 查看 HIR

```bash
rustc +nightly -Z unpretty=hir src/main.rs
```

### 查看 MIR

```bash
rustc +nightly -Z mir-opt-level=0 --emit=mir src/main.rs
```

### 查看 LLVM IR

```bash
rustc --emit=llvm-ir src/main.rs
```

### 查看宏展开

```bash
cargo expand  # 需要 cargo-expand 插件
```

### 查看编译时间分解

```bash
RUSTC_BOOTSTRAP=1 rustc -Z time-passes src/main.rs
```

---

## 延伸阅读

- [MIR 与借用检查器算法深度解析](mir_borrow_check.md)

## 设计哲学总结

### rustc 的复杂度是一种"主动的工程投资"

| 复杂度来源 | 投资目标 | 回报 |
|----------|---------|------|
| 多阶段 IR | 分离关注点 | 每个阶段可独立优化和验证 |
| MIR + 借用检查 | 编译期内存安全 | 运行时零成本 + 无 GC |
| 类型系统 + trait | 表达力 | 零成本抽象 |
| 查询系统 | 增量编译 | 大型项目迭代效率 |
| LLVM 后端 | 代码质量 | 工业级优化和跨平台支持 |

### 与 Java/Go 的根本差异

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 内存安全保证 | **编译期**（借用检查器） | **运行时**（GC + NullPointerException） | **运行时**（GC + panic） |
| 抽象成本 | **零**（单态化） | 有（泛型擦除、装箱、虚调用） | 有（interface 动态分发） |
| 编译器职责 | **语言前端 + 内存安全验证 + 优化** | 语法/类型检查 + bytecode 生成 | 语法/类型检查 + 机器码生成 |
| 运行时系统 | **无**（纯机器码） | JVM（GC、JIT、类加载） | Go runtime（GC、调度器、协程） |
| 编译时间 | 较慢 | 快 | 极快 |
| 运行时性能 | 极高（C/C++ 级别） | 中等（依赖 JIT） | 中等（依赖 GC） |

**核心洞察**：Rust 编译器的"牛逼"之处在于，它把其他语言在运行时做的事情（GC、JIT、动态类型检查）全部搬到了编译期。这是一种**时间换空间**的工程哲学——用更长的编译时间换取更短的运行时间和更小的二进制体积。

---

## 为什么 Java/Go 没有 MIR？

这是用户最常问的问题之一。答案是：**不是 MIR 本身牛，而是 Rust 选择在编译期做的事情，其他语言放在了运行时**。

### Java：javac 不做内存安全验证

Java 的编译管线（`javac`）极其简单：

```
源代码 (.java)
    ↓
Parser → AST
    ↓
类型检查
    ↓
Bytecode (.class)
```

**javac 在生成 bytecode 后就不再关心内存安全了**。为什么？

1. **内存管理交给 JVM 的 GC**：对象什么时候释放，由运行时的垃圾回收器决定
2. **空指针检查交给运行时**：`NullPointerException` 在字节码执行时抛出
3. **数组越界检查交给运行时**：`ArrayIndexOutOfBoundsException` 在字节码执行时抛出

Java 运行时（JVM）确实有中间表示，但那是 **JIT 编译器**（如 HotSpot C2）在运行时将 bytecode 转为机器码时使用的：

```
Bytecode
    ↓
HIR（High-level IR）→ C2 JIT 的 IR
    ↓
LIR（Low-level IR）→ 接近机器码的 IR
    ↓
机器码
```

**关键区别**：JVM 的 HIR/LIR 用于**运行时优化**（内联、逃逸分析），不是用于**编译期内存安全验证**。Java 的内存安全是"运行时检查 + GC"，不是"编译期证明"。

### Go：SSA 用于优化，不用于内存安全

Go 的编译管线：

```
源代码 (.go)
    ↓
Parser → AST
    ↓
类型检查
    ↓
SSA（Static Single Assignment）→ 优化
    ↓
机器码
```

Go 确实有 SSA，而且 SSA 也是一种控制流图表示。**但 Go 的 SSA 只用于优化**（死代码消除、常量传播、内联），**不用于内存安全验证**。

为什么？
1. Go 的内存安全依赖**运行时 GC**
2. Go 没有所有权/借用系统，不需要分析"变量何时失效"
3. Go 的指针可以悬空（虽然 GC 保证不 double-free，但逻辑上的 dangling 仍然可能，只是不会导致 use-after-free）

### Rust 为什么"非 MIR 不可"？

Rust 选择了一条完全不同的路：**在编译期证明程序内存安全**。

这意味着编译器需要回答这些问题：
- "在第 42 行，变量 `s` 是否已经被 move 了？"
- "在第 58 行，引用 `r` 是否仍然指向有效内存？"
- "在第 73 行，是否存在同时活跃的共享借用和可变借用？"

这些问题都是关于**控制流**的——答案取决于程序的执行路径。AST（树）无法回答这些问题，因为树没有"执行顺序"的概念。只有控制流图（CFG）才能表达"如果走这条分支，变量 X 会被 move；如果走那条分支，变量 X 不会被 move"。

**MIR 的存在是为了让借用检查器能够工作**。如果 Rust 像 Java/Go 那样依赖运行时 GC，rustc 完全可以 AST → LLVM IR → 机器码，不需要 MIR。

### 一句话总结

> **MIR 不是 Rust 编译器的"炫技"，它是 Rust "编译期内存安全"承诺的必然产物。Java/Go 不需要 MIR，是因为它们把内存安全问题交给了运行时。Rust 选择了零运行时开销，所以必须在编译期解决——MIR 就是这个解决方案的工作台。**

---

## 总结

rustc 的任务之所以"这么多"，是因为它同时承担了：

1. **传统编译器的工作**：解析、类型检查、代码生成
2. **运行时系统的工作**：内存管理（借用检查）、并发安全验证（Send/Sync）
3. **优化器的工作**：常量传播、死代码消除、内联、向量化
4. **验证器的工作**：证明程序不存在数据竞争、垂悬指针、use-after-free

这些任务的叠加使得 rustc 成为现代语言中最复杂的编译器之一。但正是这种复杂度，支撑了 Rust "零成本抽象 + 内存安全"的核心承诺。
