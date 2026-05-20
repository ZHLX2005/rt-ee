# rustc 与 cargo 的区别

## 设计背景与问题域

### 为什么要分离编译器和构建工具？

Rust 选择将 **rustc**（编译器）和 **cargo**（构建工具）分离，核心原因是：**关注点分离（Separation of Concerns）**。

**rustc 的设计目标**：
- 高效的机器码生成
- 提供稳定的编译器 API（用于 IDE、构建工具的集成）
- 作为 Rust 语言的"前端"，保证语言规范的一致性

**cargo 的设计目标**：
- 解决依赖管理的复杂性（版本解析、语义化版本、lock 文件）
- 提供统一的开发者工作流（build、test、doc、publish）
- 支持 workspace 多包管理
- 构建缓存和增量编译优化

**为什么其他语言选择不同方案？**

| 语言 | 策略 | 设计选择背后的原因 |
|------|------|-------------------|
| Java | 分离式：javac（编译器）+ Maven/Gradle（构建） | JVM 字节码是平台无关的，需要独立构建工具处理跨平台打包 |
| Go | 内置式：go build（编译+构建一体化） | Go 设计哲学是"简单"，语言和工具链深度绑定，依赖管理长期缺失后才补了 go mod |
| Rust | 协作式：rustc + cargo | 编译器专注于代码生成，构建工具专注于依赖和工作流，通过 stable ABI 解耦 |

**Rust 的关键设计洞察**：Go 的内置式方案虽然简单，但导致 Go 工具链难以独立演进（直到今天 go mod 仍是事后补救）。Java 的分离式方案虽然灵活，但两个工具来自不同项目（javac 来自 Sun，maven 来自 Apache），集成度不足。Rust 的方案是让 rustc 和 cargo 来自同一个项目，但保持清晰边界。

---

## rustc：Rust 编译器架构

### 编译管线（Compilation Pipeline）

rustc 不是单一阶段的编译器，而是多阶段优化的编译器：

```
源代码 (.rs)
    ↓
Lexer（词法分析）→ Token 流
    ↓
Parser（语法分析）→ AST（抽象语法树）
    ↓
HIR（High-level IR）→ 面向编译器的中间表示
    ↓
MIR（Mid-level IR）→ 借用检查和优化
    ↓
LLVM IR → 机器无关的中间表示
    ↓
LLVM 优化 → 目标机器码
    ↓
目标文件 / 可执行文件
```

**为什么需要这么多中间表示？**

- **AST**：直接映射语法结构，用于语法分析和宏展开
- **HIR**：剔除语法糖，保留 Rust 的核心语义（所有权、生命周期）
- **MIR**：用于借用检查（borrow checking）和控制流分析，是 Rust 独有的
- **LLVM IR**：复用 LLVM 的成熟优化和代码生成能力

**Rust 编译器的一个独特设计**：MIR 是 Rust 独有的，用于在编译期进行借用检查和生命周期分析。这是 Rust 内存安全保证的核心，而其他语言通常依赖运行时检查（GC）或手动管理。

### rustc 的直接使用场景

```bash
# 编译单个文件（不推荐用于正式项目）
rustc main.rs -o main

# 指定 edition
rustc main.rs --edition 2021 -o main

# 查看编译器诊断信息
rustc main.rs 2>&1
```

**rustc 的输出**：直接生成机器码或目标文件，没有中间的包格式（对比 Java 的 .class 或 .jar）。

---

## cargo：Rust 构建系统和包管理器

### cargo 的核心职责

cargo 解决的问题远不止"调用 rustc"：

1. **依赖解析（Dependency Resolution）**
   - 语义化版本（SemVer）解析
   - 依赖图构建和冲突检测
   - Cargo.lock 锁定具体版本

2. **构建协调（Build Coordination）**
   - 并行编译独立 crates
   - 增量编译（记住之前的编译结果）
   - 构建缓存（target/ 目录）

3. **工作空间（Workspace）管理**
   - 多包项目的统一构建
   - 共享依赖（多个 crate 复用同一个依赖）

4. **开发者工作流**
   - `cargo new` / `cargo init`
   - `cargo build` / `cargo run` / `cargo test`
   - `cargo doc` / `cargo doc --open`
   - `cargo publish` / `cargo search`

### cargo 的构建缓存机制

```
target/
├── debug/              # debug 构建
│   ├── deps/           # 依赖编译结果
│   ├── examples/       # 示例
│   └── incremental/    # 增量编译缓存
├── release/            # release 构建
└── .fingerprint/       # 编译指纹（检测是否需要重新编译）
```

增量编译的核心思想：**追踪每个源文件的指纹（fingerprint），只重新编译那些源文件或依赖发生变化的 crate**。这个机制大大加速了开发迭代。

### 与 Java/Go 工具链的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **编译器输入** | 源代码（.rs） | 源代码（.java） | 源代码（.go） |
| **编译器输出** | 机器码/目标文件 | JVM 字节码（.class） | 机器码（直接链接） |
| **运行时依赖** | 无（静态链接） | JVM | 无（静态链接） |
| **包格式** | crate（.a/.rlib） | .jar / .war | package（无物理格式） |
| **构建工具** | cargo | maven / gradle | go build / go mod |
| **依赖源** | crates.io | maven central / jcenter | proxy.golang.org |
| **锁文件** | Cargo.lock | pom.xml / gradle.lockfile | go.sum |
| **工作空间** | Cargo workspace | maven multi-module | go workspace（2024+） |

**关键差异解读**：

1. **编译产物形态**：
   - Rust：静态链接的可执行文件，无需运行时依赖
   - Java：字节码，需要 JVM 运行（"write once, run anywhere"的代价）
   - Go：静态链接，但编译器输出直接可执行

2. **依赖管理进化路径**：
   - Rust（2015）：从一开始就设计了 cargo，具备完整的依赖管理
   - Go（2009-2020）：长期没有官方依赖管理，go mod 是 2020 年才加入的"亡羊补牢"
   - Java（1995-现在）：从 javac 开始就支持分离构建，但依赖管理（maven 2004）晚于编译器

3. **构建哲学**：
   - Rust：cargo 是" Batteries included"的一部分，但编译器保持独立
   - Go：语言和工具链深度耦合，"go" 命令无处不在
   - Java：编译器 javac 和构建工具完全分离（来自不同项目）

---

## rustc 与 cargo 的协作关系

### cargo 如何调用 rustc

cargo 不会直接暴露 rustc 的所有选项，而是通过 `RUSTFLAGS` 和 `--cap-lints` 等机制控制编译行为：

```bash
# cargo 底层调用的 rustc 示例
rustc --edition 2021 --crate-type bin --emit=link -o target/debug/my_app src/main.rs

# 通过 RUSTFLAGS 传递额外参数
RUSTFLAGS="-C opt-level=3" cargo build --release
```

### Cargo.toml 的角色

Cargo.toml 是 cargo 的配置格式，也是 crate 的"清单（manifest）"：

```toml
[package]
name = "my_crate"          # 包名
version = "0.1.0"          # 语义化版本
edition = "2021"           # Rust edition

[dependencies]
serde = "1.0"              # 精确版本
tokio = "1.0"              # caret 约束（^1.0，即 >=1.0, <2.0）
anyhow = "1.4"             # 补丁版本兼容

[dev-dependencies]         # 仅测试环境使用
tempfile = "3.0"

[build-dependencies]      # 构建脚本依赖
prost-build = "0.10"
```

**Cargo.toml 的设计意图**：声明式依赖而非命令式脚本。这与 Maven/Gradle 的 pom.xml 或 build.gradle 不同——cargo 不执行任意的构建脚本（除了 build.rs），而是通过声明式配置驱动构建。

---

## 设计哲学总结

### 为什么 Rust 选择"独立编译器 + 统一构建工具"？

**1. 编译器的稳定性需求**
rustc 作为 Rust 语言的唯一编译器，必须保持极度稳定。IDE、构建工具都依赖 rustc 的 stable API。如果构建逻辑和编译逻辑耦合在一起，升级构建工具可能导致编译器行为变化。

**2. 工具链的独立演进**
cargo 可以独立于 rustc 演进（只要保持最低支持的 rustc 版本）。这意味着：
- 依赖解析算法可以快速迭代
- 新功能（workspace、rustflags）可以快速添加
- 不需要为了构建工具的改动而重新发布 Rust stable

**3. 对比其他语言的权衡**
- **Go 的教训**：go build 深度耦合在语言中，导致 go mod 来晚了10年，且至今工具链仍在"补全"中
- **Java 的教训**：javac 和 maven 来自不同项目，集成度差，IDE 支持需要额外插件

**Rust 的答案**：编译器 rustc 和构建工具 cargo 来自同一个项目（rust-lang/rust），但保持清晰的接口边界。cargo 调用 rustc 时使用稳定的命令行接口，而不是内部 API。

### cargo 的设计亮点

1. **语义化版本（SemVer）**：
   - Cargo.toml 声明的是"接口兼容承诺"
   - cargo 自动解析依赖时遵循 SemVer 约束
   - 这是 Rust 生态健康的基础

2. **Lock 文件（Cargo.lock）**：
   - 精确记录每个依赖的具体版本
   - 确保"可重现构建"（reproducible build）
   - 建议提交到版本控制

3. **Workspace**：
   - 单仓库多包（monorepo）的 Rust 解决方案
   - 共享 target/ 目录，节省磁盘空间
   - 统一版本号管理

---

## 延伸阅读

- [Rust 编译器管线深度解析](compiler_pipeline/compiler_pipeline.md)

## 常见用法速查

| 命令 | 说明 | rustc 等价 |
|------|------|-----------|
| `cargo new project` | 创建新项目 | - |
| `cargo init` | 初始化现有目录 | - |
| `cargo build` | 编译项目 | `rustc src/main.rs` |
| `cargo build --release` | Release 构建 | `rustc -C opt-level=3 src/main.rs` |
| `cargo run` | 运行项目 | `rustc && ./project` |
| `cargo test` | 运行测试 | `rustc --test` |
| `cargo doc --open` | 生成并查看文档 | `rustdoc` |
| `cargo add crate` | 添加依赖 | 手动编辑 Cargo.toml |
| `cargo publish` | 发布到 crates.io | - |
| `cargo tree` | 显示依赖树 | - |
| `cargo check` | 检查代码（不生成目标文件） | `rustc --emit=metadata` |

---

## 运行示例

```bash
# 克隆一个 Rust 项目并构建
git clone https://github.com/rust-lang/rustlings
cd rustlings
cargo install --path .          # 从源码安装（cargo 会自动调用 rustc）
cargo build                      # 编译项目
cargo run                        # 运行
```
