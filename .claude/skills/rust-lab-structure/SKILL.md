---
name: rust-lab-structure
description: 当用户要求创建rust学习目录结构、添加新的rust功能验证模块、或需要管理lab和docs时触发
---

# Rust Lab 目录结构管理

## 核心原则

**用户是 Java/Go 程序员，需要从 Rust 获得高端抽象语言设计启发。**

文档和代码不仅要教"怎么做"，更要讲清楚"为什么这样设计"——Rust 的每一个特性背后都有深刻的设计哲学。

---

## 目录结构规范

```
rt-ee/
├── lab/                    # 可独立运行的 Rust 代码
│   ├── {module}/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── ...
├── docs/                    # 文档（知识 + 代码）
│   ├── README.md
│   └── {module}/
│       └── {module}.md     # 必须有同名 md
└── README.md
```

## 约束

| 约束 | 说明 |
|------|------|
| `lab/{module}/` | 每个模块独立的 Cargo.toml + src/main.rs |
| `docs/{module}/` | 每个模块一个目录，目录内必须有 `{module}.md` |
| 模块名 | 只用小写字母和下划线 |
| 同步创建 | 创建 lab 模块时必须同步创建 docs 文档 |
| 知识记录 | 用户提问的知识点也必须记录到 docs |

---

# 场景一：添加新模块

当用户说"添加 xxx 模块"、"学习 xxx"、"我想验证 xxx" 时：

## Step 1: 确定模块名

将用户需求转换为小写下划线格式，例如：
- "所有权" → `ownership`
- "生命周期" → `lifetimes`
- "rustc和cargo的区别" → `rustc_vs_cargo`

## Step 2: 分析设计意图

**每个 Rust 特性都要回答：**
1. 这个特性解决了什么问题？
2. 为什么其他语言（Java/Go）用不同方式解决？
3. Rust 的设计决策背后有什么权衡？

## Step 3: 创建 lab/{module}（如有代码）

```bash
mkdir -p lab/{module}/src
```

## Step 4: 创建 Cargo.toml

```toml
[package]
name = "{module}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{module}"
path = "src/main.rs"
```

## Step 5: 创建 src/main.rs

代码要展示**设计意图**，不只是语法验证：

```rust
// 注释要解释：为什么要这样做，而不是仅仅展示"这样写能编译"
// 对比 Java/Go 的做法，说明 Rust 的设计决策
```

## Step 6: 创建 docs/{module}/{module}.md

**重要**：无论是否有代码，知识性问题也必须创建文档！

### 文档深度策略

| 知识状态 | 策略 | 说明 |
|---------|------|------|
| 单一核心概念，内聚完整 | 单文档，深度覆盖 | 文档可以很长，但必须讲透 |
| 多个关联概念 | 多文档 a/b/c... | 每个文档独立深度覆盖一个子主题 |
| 主文档作为索引 | {module}.md 汇总 | 各子文档链接互通 |

**核心原则**：高内聚优于多文档。浅薄的单文档不如深厚的单文档。如果拆分为多文档，每个文档必须独立完整。

### 文档章节要求

```markdown
# {Module} 模块

## 设计背景与问题域

这个特性解决什么问题？为什么 Rust 要这样设计？
与其他语言（Java/Go）的方案有什么本质区别？

## 抽象设计分析

这个特性体现了 Rust 的哪些设计原则？
类型系统/所有权/借用在这其中如何协作？

## 核心规则

用精确的语言描述规则，而不是表面的"语法"。

## 代码示例（带设计意图注释）

每个示例要包含：
1. 完整可运行的代码
2. 逐行注释：为什么这样写，而非那样写
3. 编译器输出的错误信息及解释
4. Java/Go 等价实现的对比（如果有意义）
5. 常见误区与正确模式的对比

## 设计决策对比表

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 内存管理 | ... | ... | ... |
| 并发安全 | ... | ... | ... |
| 类型系统 | ... | ... | ... |

## 运行（如果有代码）

```bash
cargo run -p {module}
```
```

## Step 7: 更新 docs/README.md

在目录列表中添加新模块链接。

---

# 场景二：知识问答记录

当用户提问 Rust 知识点时（如 rustc vs cargo、所有权规则等）：

## 前置原则（关键！）

**先归档，再回答！**

1. **检查**：用户提问的主题是否在 `docs/` 中有对应文档？
2. **归档**：
   - 如果**没有**对应文档 → **必须先创建文档**，再回答用户
   - 如果**已有**对应文档 → 复用已有文档，在回答中引用
3. **回答**：归档完成后，再回答用户问题

> **常见错误**：只口头回答用户的问题，不创建/更新文档。这导致知识散落，后续无法追溯。

## 执行步骤

1. **确定模块名** → 如 `rustc_vs_cargo`
2. **检查 docs/ 是否有对应文档** → 如 `docs/rustc_vs_cargo/rustc_vs_cargo.md`
3. **分析设计意图** → 回答"Why"层面的问题
4. **创建或更新 docs/{module}/{module}.md** → 整理知识内容（无论是否已有）
5. **更新 docs/README.md** → 添加链接
6. **回答用户** → 引用文档位置

## 文档格式要求

```markdown
# rustc 与 cargo 的区别

## 设计背景

为什么要分离 rustc 和 cargo？这样做有什么好处？

## rustc 的设计职责

编译器负责什么？为什么 Rust 选择直接输出机器码？

## cargo 的设计职责

构建系统负责什么？为什么需要独立的构建工具？

## 与 Java/Go 的工具链对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 编译器 | rustc | javac | go build |
| 构建工具 | cargo | maven/gradle | go mod |
| 包管理 | crates.io | maven central | proxy.golang.org |

## 设计哲学

为什么 Rust 选择"独立编译器 + 统一构建工具"而非 Java 的分离式或 Go 的内置式？
```

---

# 内容深度分级

## Level 1: 语法验证（禁止只做这个）

```rust
// 错误示范：只验证语法
let s1 = String::from("hello");
let s2 = s1;
println!("{}", s2);
```

## Level 2: 设计意图（至少做到这个）

```rust
// 设计意图：所有权转移语义
// 为什么 Rust 选择 move 而不是 GC/引用计数？
// - 确定性析构：无运行时开销
// - 所有权清晰：编译器就能推断何时释放
// 对比 Java：GC 的非确定性 vs Rust 的确定性析构
let s1 = String::from("hello");
let s2 = s1; // s1 无效，所有权转移给 s2
println!("{}", s2); // s2 负责在离开作用域时释放
```

## Level 3: 抽象设计启发（目标）

```rust
// 抽象设计：所有权是一种线性类型系统
// 思考：为什么线性类型适合资源管理？
// - 资源 = 内存、文件、锁等
// - 线性：每个资源必须有且只有一个 owner
// - Owner 负责释放 → 确定性资源管理
// 这种设计在 Java/Go 中如何实现？有何取舍？
```

## Level 4: 高级工程师深度（目标）

```rust
// 高级工程师视角：不只是"能跑"，而是"为什么这样设计"
// 所有权转移的背后：线性类型的资源管理哲学
//
// Rust 的 move 语义是线性类型理论的具体实现：
// - 线性类型要求每个值必须被使用且仅使用一次
// - Rust 通过所有权系统在编译期强制这一约束
// - 对比：Java 依赖 GC + 引用计数（不精确），Go 依赖 GC（更不精确）
//
// 关键洞察：Rust 的确定性析构不是"特性"，而是"线性类型的必然结果"
// 因为每个资源必须有唯一 owner，而 owner 负责释放，所以释放时机是确定的

fn main() {
    // 演示：为什么 String 而非 &str？
    // String 拥有所有权，&str 只是借用
    // 这意味着 String 可以直接控制资源的生命周期
    let s1 = String::from("hello");
    let s2 = s1; // move：所有权转移，s1 无效
    // s1 在此处无法使用，编译器会阻止
    println!("{}", s2);

    // 对比：Java 的 String 和 Go 的 string
    // Java: String s1 = "hello"; String s2 = s1; // s1 仍然有效，共享引用
    // Java 需要 GC 来回收不再引用的字符串
    //
    // Go: s1 := "hello"; s2 := s1 // s1 仍然有效，字符串不可变，共享底层数组
    // Go 的字符串不可变，但仍然依赖 GC

    // Rust 的优势：
    // 1. 编译期确定释放时机，无 GC 暂停
    // 2. 双重引用计数（Arc/Rc）需要显式选择，不是默认
    // 3. 借用检查器防止垂悬指针和数据竞争
}
```

---

# 示例对话

**用户**: "rustc和cargo的区别是什么"

**AI 执行**:
1. 模块名: `rustc_vs_cargo`
2. 分析设计意图：为什么分离？分离的好处？
3. `mkdir -p docs/rustc_vs_cargo`
4. 创建 `docs/rustc_vs_cargo/rustc_vs_cargo.md`
   - 包含设计背景、职责划分、与 Java/Go 工具链对比
   - rustc 内部架构：Lexer → Parser → AST → HIR → MIR → LLVM → 目标文件
   - cargo 的构建图、依赖解析、workspace 机制
   - 编译期插件和过程宏的集成方式
   - 与 javac + Maven/Gradle、go build + go mod 的深度对比
5. 更新 `docs/README.md`

---

**用户**: "我想学习 rust 的并发编程"

**AI 执行**:
1. 模块名: `concurrency`
2. 分析设计意图：
   - Rust 并发模型的核心问题：数据竞争
   - 为什么选择"编译时检查"而非"运行时检查"？
   - Send/Sync trait 的设计哲学
3. `mkdir -p lab/concurrency/src`
4. 创建 `lab/concurrency/Cargo.toml`
5. 创建 `lab/concurrency/src/main.rs`
   - 展示 Arc/Mutex 的设计意图
   - 对比 Java 的 synchronized 和 Go 的 channel
   - channel 内部实现：MPSC 队列、阻塞语义
   - Send/Sync 的标记语义与 auto trait 机制
6. 创建 `docs/concurrency/concurrency.md`
   - 包含设计背景、抽象分析、与 Java/Go 对比
   - 包含完整的代码示例和编译器错误信息解读
   - 包含锁竞争、死锁预防、内存顺序等高级主题
7. 更新 `docs/README.md`

---

# 坑点警示

| 错误操作 | 实际后果 | 正确做法 |
|---------|---------|---------|
| docs 直接放 md 文件 | 违反规范 | 必须 `docs/{module}/{module}.md` |
| 知识问答没记录文档 | 知识点丢失 | 所有用户提问都必须记录到 docs |
| 用户问新主题，只口头回答不归档 | 知识散落，无法追溯 | **先创建/更新文档，再回答用户** |
| 问题与现有文档重叠，不引用已有文档 | 用户看不到完整知识 | 复用并引用已有文档 |
| lab 没有独立 Cargo.toml | 模块无法单独运行 | 每个 lab/{module} 需独立 Cargo.toml |
| 忘记同步创建 docs | 代码和文档脱节 | lab 和 docs 必须同步创建 |
| 模块名用大写 | cargo 报错 | 只用小写字母和下划线 |
| 只写语法验证代码 | 用户只学会怎么写，不知道为什么 | 代码必须包含设计意图注释 |
| 文档只有"概念说明" | 缺少设计深度 | 必须包含设计背景、与 Java/Go 对比 |
| 浅薄的单文档 | 高内聚知识被割裂 | 宁可厚文档，不要薄文档 |
| 多文档但每文档都浅 | 知识碎片化 | 拆分的前提是每文档独立完整 |
