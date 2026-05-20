# Rust 宏机制与 Tauri 2 前后端宏分析

## 设计背景与问题域

Java 用注解（Annotation）+ 反射/注解处理器实现元编程。Go 用 `go generate` 在编译前生成代码。Rust 的选择是**宏**——在编译期直接操作 AST 的元编程机制。

为什么 Rust 不用 Java 的注解模型？

| 维度 | Rust 宏 | Java 注解 |
|------|--------|----------|
| 执行时机 | 编译期 | 编译期处理器或运行期反射 |
| 输入 | Token Stream（AST） | AST 元素 |
| 输出 | 新的 Token Stream（参与类型检查） | 生成新源文件或运行时元数据 |
| 类型安全 | 生成的代码必须类型正确 | 生成后类型检查 |
| 运行时成本 | 零 | 反射有成本 |
| IDE 支持 | rust-analyzer 可展开宏 | 注解处理器结果可被识别 |

Rust 宏的核心设计原则：**编译期代码生成，零运行时成本**。

---

## Rust 宏的两种形态

### 1. 声明式宏（macro_rules!）

基于**模式匹配**的代码模板替换。类比 C 的宏，但更安全（操作的是 Token 树而非纯文本）。

```rust
macro_rules! my_vec {
    () => { Vec::new() };
    ($elem:expr; $n:expr) => { /* ... */ };
    ($($x:expr),+) => { /* ... */ };
}
```

**关键机制**：
- 模式匹配从左到右，第一个匹配的分支生效
- `$name:kind` 捕获 Token（expr = 表达式，ty = 类型，ident = 标识符）
- `$($x:expr),*` 重复捕获（零次或多次）
- 宏展开是** hygienic**（卫生的）：宏内部生成的变量不会污染外部作用域

### 2. 过程宏（Procedural Macros）

用 Rust 代码编写编译期变换函数，直接操作 Token Stream。三种类型：

| 类型 | 语法 | 用途 |
|------|------|------|
| Derive 宏 | `#[derive(X)]` | 为 struct/enum 自动生成 impl |
| Attribute 宏 | `#[x(...)]` | 修饰函数/结构体/模块，变换其 AST |
| Function-like 宏 | `x!(...)` | 类似声明式宏，但用 Rust 代码实现 |

---

## 过程宏的实现原理

### 核心组件

```rust
use proc_macro::TokenStream;

#[proc_macro_derive(Builder)]
pub fn derive_builder(input: TokenStream) -> TokenStream {
    // input: 被 derive 的 struct/enum 的原始 Token
    // 返回：新生成的 Token（如 Builder 结构体和方法）
}
```

**syn 库**：将 TokenStream 解析为强类型的 AST 节点
**quote 库**：将 Rust 代码模板重新生成为 TokenStream

```rust
// 典型流程
let ast = syn::parse(input)?;           // Token -> AST
let generated = quote! { /* ... */ };   // AST -> Token
TokenStream::from(generated)
```

### 关键约束

- 过程宏 crate **必须独立**，`proc-macro = true`
- 过程宏 crate **不能导出普通类型/函数**
- 过程宏在编译期执行，**不能访问运行时状态**（如文件系统、网络）
- 但可以读取编译时环境变量和 `CARGO_*` 变量

---

## Tauri 2 的宏机制分析

Tauri 是一个 Rust 后端 + Web 前端的跨平台桌面应用框架。其前后端通信大量依赖宏来消除样板代码。

### 1. `#[tauri::command]` — 后端命令暴露

```rust
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}
```

**宏做了什么？**

```rust
// 宏展开后的等价代码（简化）
fn greet(name: &str) -> String {
    format!("Hello, {}!", name)
}

// 自动生成序列化/反序列化包装器
fn greet_invoke(
    invoke: tauri::Invoke,
) {
    let resolver = invoke.resolver;
    let window = invoke.message.window();
    let payload = invoke.message.payload();

    // 自动从 JSON payload 中提取参数
    let name: String = serde_json::from_value(payload["name"].clone()).unwrap();

    // 调用实际函数
    let result = greet(&name);

    // 自动将返回值序列化为 JSON 并通过 IPC 发送回前端
    resolver.resolve(result);
}
```

**设计价值**：
- 开发者只需写纯 Rust 函数，无需手动处理 JSON 序列化和 IPC
- 类型安全：参数和返回值的类型在编译期确定
- 零运行时开销：序列化代码在编译期生成，无反射

### 2. `tauri::generate_handler![]` — 命令注册

```rust
tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![greet, get_user, save_file])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
```

**宏做了什么？**

这是一个**声明式宏**，生成命令分发路由表：

```rust
// 展开后的等价代码（简化）
tauri::Builder::default()
    .invoke_handler(
        // 生成一个闭包，根据命令名分发给对应的函数
        |invoke| {
            match invoke.message.command() {
                "greet" => greet_invoke(invoke),
                "get_user" => get_user_invoke(invoke),
                "save_file" => save_file_invoke(invoke),
                _ => invoke.resolver.reject("Unknown command"),
            }
        }
    )
```

**设计价值**：
- 命令名到函数的映射在编译期确定（字符串匹配）
- 新增命令只需在数组中添加，无需手动维护路由表
- 如果命令名拼写错误，编译期就能发现（因为只生成存在的函数的 invoke 包装器）

### 3. `tauri::generate_context!()` — 资源配置

```rust
tauri::generate_context!()
```

这个宏读取项目中的 `tauri.conf.json`，在编译期将其内容嵌入到二进制中。这样运行时无需读取外部配置文件。

### 4. `#[mobile_entry_point]` — 移动端入口

Tauri 2 支持 iOS/Android，这个属性宏为移动端生成平台特定的入口代码：

```rust
#[cfg_attr(mobile, tauri::mobile_entry_point)]
fn main() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .unwrap();
}
```

**宏做了什么？**
- 在移动端目标平台，生成 `UIApplicationMain`（iOS）或 `android_main`（Android）包装器
- 在桌面端，保持普通的 `main()` 函数

---

## 宏机制的设计对比

| 维度 | Rust 宏 | Java 注解处理器 | Go generate |
|------|--------|---------------|-------------|
| 触发方式 | `#[derive]` / `macro!` | 注解 + 编译器插件 | `//go:generate` 注释 |
| 执行时机 | 编译期（类型检查前） | 编译期（特定轮次） | 编译前（手动触发） |
| 输入 | Token Stream | AST 元素 | 任意文件 |
| 输出 | Token Stream（参与类型检查） | 新源文件 | 新源文件 |
| 可重复性 | Cargo 自动保证 | 编译器保证 | 依赖开发者手动运行 |
| 运行时反射 | 不需要 | 可能需要 | 不需要 |
| 错误位置 | 宏调用处 | 生成的代码处 | 生成的代码处 |

---

## 工程实践：何时使用哪种宏

| 场景 | 推荐方案 | 原因 |
|------|---------|------|
| 简单的代码重复（vec!, println!） | 声明式宏 | 简洁，无需额外 crate |
| 为 struct 生成样板代码（Builder, Serialize） | Derive 过程宏 | 类型驱动，编译期展开 |
| 修饰函数/模块（路由、日志、测试） | Attribute 过程宏 | 最小侵入，声明式 |
| 复杂 DSL（HTML, SQL 模板） | Function-like 过程宏 | 完全控制语法 |

---

## 延伸阅读

- [Rust 过程宏 vs Java 注解处理器：AST 操作深度对比](proc_macro_vs_annotation.md)

---

## 运行示例

```bash
# 运行声明式宏 + 过程宏演示
cargo run -p procedural_macros
```

### 项目结构说明

```
lab/
├── procedural_macros_derive/    # 过程宏定义 crate（proc-macro = true）
│   ├── Cargo.toml
│   └── src/lib.rs               # #[derive(Builder)] 和 #[trace_function] 的实现
└── procedural_macros/           # 使用过程宏的主程序
    ├── Cargo.toml
    └── src/main.rs              # 演示宏的使用
```

**关键设计**：过程宏 crate 必须与普通代码 crate 分离，因为过程宏在编译期执行，运行环境完全不同。
