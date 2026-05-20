# Rust 工程管理的高级框架

## 设计背景与问题域

Java 有 Maven/Gradle 的多模块项目、Spring Boot 的 starter 依赖、切面编程。Go 有 go modules、workspace（1.18+）、go generate。Rust 的工程管理体系与它们有本质不同——**Cargo 不仅是构建工具，更是工程约束的声明系统**。

Rust 工程管理要解决的核心问题：

1. **单体仓库（mono-repo）如何管理多个 crate？** — Cargo Workspace
2. **如何在编译期条件化功能和依赖？** — Features 系统
3. **如何在构建阶段执行自定义逻辑？** — Build Scripts
4. **语言如何在不破坏兼容性的情况下演进？** — Edition 系统
5. **如何防止供应链攻击和不安全代码？** — 工具链审计

---

## 一、Cargo Workspace：Rust 的 mono-repo 方案

### 与 Java/Go 的对比

| 维度 | Rust Workspace | Maven Multi-Module | Go Workspace |
|------|---------------|-------------------|-------------|
| 模块单元 | Crate（编译单元） | Module（逻辑单元） | Module（逻辑单元） |
| 编译缓存 | 跨 crate 共享 | 部分支持 | 模块缓存 |
| 依赖统一 | 根 Cargo.toml 统一管理 | parent POM | go.work |
| 版本管理 | 各 crate 独立版本 | parent 统一管理 | 模块独立 |
| 发布 | 各 crate 独立发布到 crates.io | 统一或独立 | 独立 |

### Workspace 结构

```
my-project/
├── Cargo.toml          # workspace 根
├── crates/
│   ├── core/           # 核心库
│   │   ├── Cargo.toml
│   │   └── src/lib.rs
│   ├── server/         # 服务端
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   └── client/         # 客户端
│       ├── Cargo.toml
│       └── src/main.rs
└── shared-deps.toml    # 可引入的依赖配置
```

```toml
# 根 Cargo.toml
[workspace]
members = ["crates/*"]
resolver = "2"  # 依赖解析器版本

# 全局依赖版本锁定（可选）
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }

# 根 package 可以没有，也可以有
[package]
name = "my-project-meta"
version = "0.1.0"
```

```toml
# crates/server/Cargo.toml
[package]
name = "server"
version = "0.1.0"

[dependencies]
# 继承 workspace 依赖版本
core = { path = "../core" }
tokio = { workspace = true }
serde = { workspace = true }
```

### Workspace 的核心优势

1. **统一编译缓存**：所有 crate 共享 target/ 目录，增量编译跨 crate 生效
2. **原子改动**：修改 core API，server 和 client 的编译错误同时暴露
3. **依赖去重**：workspace 中相同的依赖只编译一次
4. **统一测试**：`cargo test --workspace` 运行所有 crate 的测试

---

## 二、Features 系统：编译期条件化的工程艺术

### Features 不是简单的条件编译

Rust 的 features 是**依赖管理 + 条件编译 + API 契约**的统一体。

```toml
[features]
# 默认启用的 features
default = ["std", "derive"]

# 条件编译标志
std = []                          # 标准库支持
no_std = []                       # 嵌入式支持（互斥）
derive = ["dep:serde", "serde?/derive"]  # 启用 serde 的 derive feature
async = ["dep:tokio", "dep:futures"]     # 可选异步依赖

# 互斥特征（运行时选择）
backend-rocksdb = ["dep:rocksdb"]
backend-sled = ["dep:sled"]
```

```rust
// 代码中条件编译
#[cfg(feature = "std")]
pub fn std_only_function() { }

#[cfg(not(feature = "std"))]
pub fn no_std_function() { }

// 依赖的条件启用
#[cfg(feature = "async")]
use tokio::runtime;
```

### 与 C/C++ 条件编译的本质区别

| 维度 | Rust Features | C #ifdef |
|------|--------------|----------|
| 类型安全 | 条件编译的代码参与类型检查 | 预处理器文本替换 |
| 依赖管理 | features 控制依赖是否链接 | 手动链接 |
| API 稳定性 | 特征组合经 CI 测试 | 组合爆炸，难以测试 |
| 包管理 | crates.io 包包含 features 元数据 | 无 |

### Features 的组合爆炸问题

```bash
# 测试所有 feature 组合（使用 cargo-hack）
cargo hack test --feature-powerset --depth 2

# 只测试重要组合
cargo test --no-default-features
cargo test --all-features
cargo test --features "async,derive"
```

---

## 三、Build Scripts：编译期元编程

### build.rs 的工程用途

```rust
// build.rs
use std::env;
use std::path::Path;

fn main() {
    // 1. 链接系统库
    println!("cargo:rustc-link-lib=ssl");
    println!("cargo:rustc-link-search=/usr/local/lib");

    // 2. 代码生成（protobuf、接口定义）
    let out_dir = env::var("OUT_DIR").unwrap();
    // 生成 src 到 out_dir，再用 include!() 引入

    // 3. 编译 C/C++ 依赖
    cc::Build::new()
        .file("src/native/helper.c")
        .compile("helper");

    // 4. 环境探测（feature 可用性）
    println!("cargo:rustc-cfg=has_native_tls");
}
```

```rust
// src/lib.rs
include!(concat!(env!("OUT_DIR"), "/generated.rs"));

#[cfg(has_native_tls)]
pub fn native_tls() { }
```

### 与 Go generate 的对比

| 维度 | Rust build.rs | Go generate |
|------|--------------|-------------|
| 执行时机 | 编译前自动执行 | 需手动运行 `go generate` |
| 输入 | Cargo.toml 依赖、环境变量 | 源代码注释 `//go:generate` |
| 输出 | 编译产物、条件编译标志 | 新的 Go 源文件 |
| 可重复性 | 由 Cargo 保证 | 依赖开发者手动执行 |

---

## 四、Edition 系统：语言的向后兼容演进

### 什么是 Edition？

Rust 通过 **Edition** 机制在不破坏现有代码的情况下引入不兼容的语法改进。

```toml
[package]
name = "my-crate"
edition = "2021"  # 可选：2015, 2018, 2021, 2024
```

**关键设计**：
- 同一编译单元内，所有代码使用同一 edition
- 不同 edition 的 crate 可以无缝互操作
- 编译器同时支持所有 edition

### Edition 演进示例

| Edition | 主要变化 |
|---------|---------|
| 2015 | 原始版本 |
| 2018 | `async/await`、模块路径简化、NLL（非词法生命周期） |
| 2021 | 闭包捕获规则改进、panic 默认一致性、预留字 |
| 2024 | 尾表达式临时值生命周期、UnsafeOpInUnsafeFn |

**对比 Java**：Java 8/11/17 的语法变化需要 JVM 版本配合，Rust edition 完全由编译器处理，不依赖运行时。

---

## 五、过程宏：工程化的代码生成基础设施

### 三类过程宏的工程定位

```rust
// 1. Derive 宏：消除样板代码
#[derive(Debug, Clone, Serialize, Deserialize)]
struct User { id: u64, name: String }

// 2. Attribute 宏：声明式编程
#[tokio::main]                    // 生成 async main 包装器
#[derive_builder::Builder]        // 生成 Builder 模式代码
#[tracing::instrument]            // 自动插入日志和 span

// 3. Function-like 宏：DSL
let doc = html! {
    <div class="container">
        <h1>{title}</h1>
    </div>
};
```

### 过程宏的工程价值

| 场景 | 手写代码 | 过程宏方案 |
|------|---------|-----------|
| JSON 序列化 | 100+ 行 impl | `#[derive(Serialize)]` |
| Builder 模式 | 50+ 行 setter | `#[derive(Builder)]` |
| HTTP 路由 | 手动注册 | `#[get("/users")]` |
| SQL 校验 | 运行时错误 | `sql!(SELECT * FROM users)`（编译期检查） |

---

## 六、工具链生态：超越编译器的工程保障

### 核心工具矩阵

| 工具 | 用途 | 对应 Java/Go 生态 |
|------|------|------------------|
| `rustfmt` | 代码格式化 | google-java-format, gofmt |
| `clippy` | 静态分析（400+ lint） | SpotBugs, golangci-lint |
| `cargo-deny` | 依赖审计（许可证、安全） | OWASP Dependency-Check |
| `cargo-audit` | 已知漏洞扫描 | Snyk, Dependabot |
| `cargo-outdated` | 依赖版本检查 | Versions Maven Plugin |
| `miri` | 未定义行为检测 | 无直接对应 |
| `cargo-fuzz` | 模糊测试 | JQF, go-fuzz |
| `cargo-bench` | 性能基准 | JMH, go test -bench |

### Clippy：超越 lint 的工程约束

```rust
// Clippy 可以阻止这些模式：

// 1. 隐式拷贝大类型
let v = vec![0; 1000];
let v2 = v; // OK: move
// let v2 = v.clone(); // Clippy: large type passed by value

// 2. 不必要的引用解引用
let x = &&5;
// let y = **x; // Clippy: explicit deref

// 3. 可 panic 的算术
// let a = 1 / 0; // Clippy: integer division by zero
```

**工程实践**：将 clippy 的 `deny` 级别规则纳入 CI，作为代码合并的门禁。

---

## 七、大型 Rust 项目的架构模式

### Crate 分层架构

参考 rustc、tokio、rust-analyzer 的组织方式：

```
project/
├── crates/
│   ├── core/              # 纯逻辑，无 IO，无 async
│   ├── domain/            # 业务类型和规则
│   ├── infrastructure/    # 数据库、HTTP、文件系统适配
│   ├── application/       # 用例编排（应用服务）
│   └── server/            # 可执行入口（main.rs）
├── tests/                 # 集成测试（跨 crate）
└── benches/               # 性能测试
```

**核心原则**：
- `core` 和 `domain` 不依赖任何外部 crate（或极少依赖）
- 依赖方向：`server` → `application` → `infrastructure` → `domain` → `core`
- 禁止循环依赖（Cargo 强制保证）

### 公开 API 设计

```rust
// crates/core/src/lib.rs

// 公开类型
pub use self::parser::Parser;
pub use self::error::Error;

// 内部模块不公开
mod parser;
mod error;
mod internal_utils;  // 私有

// 公开 trait，但隐藏实现细节
pub trait Compile {
    fn compile(&self) -> Result<Artifact, Error>;
}

// 使用 #[non_exhaustive] 保护未来扩展
#[non_exhaustive]
pub enum Error {
    Parse,
    TypeCheck,
}
```

---

## 与 Java/Go 工程管理的总对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 构建工具 | Cargo（编译+测试+文档+发布一体） | Maven/Gradle | go build + go test |
| 工作空间 | Workspace | Multi-module | Workspace (1.18+) |
| 条件编译 | Features 系统 | Maven Profiles | Build Tags |
| 代码生成 | 过程宏（编译期） | 注解处理器 | go generate |
| 包仓库 | crates.io | Maven Central | proxy.golang.org |
| 格式化 | rustfmt（标准） | 多种风格 | gofmt（标准） |
| 静态分析 | clippy（官方） | SpotBugs, PMD | golangci-lint |
| 依赖审计 | cargo-deny, cargo-audit | OWASP | govulncheck |
| 模糊测试 | cargo-fuzz（官方支持） | JQF | go-fuzz |
| 语言演进 | Edition 系统 | JVM 版本 | 无（向后兼容优先） |

---

## 总结

Rust 的工程管理体系有几个独特的设计：

1. **Workspace + Crate**：编译单元即模块边界，依赖方向由编译器强制
2. **Features 系统**：将条件编译、可选依赖、API 契约统一为声明式配置
3. **过程宏**：编译期代码生成，消除样板代码的同时保持类型安全
4. **Edition 系统**：语言演进的平滑路径，没有"Python 2/3 分裂"
5. **工具链完整性**：从格式化到模糊测试，官方提供端到端的工程保障

这些机制共同构成了一套**声明式、可验证、可演进**的工程管理框架。
