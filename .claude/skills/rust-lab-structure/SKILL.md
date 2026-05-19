---
name: rust-lab-structure
description: 当用户要求创建rust学习目录结构、添加新的rust功能验证模块、或需要管理lab和docs时触发
---

# Rust Lab 目录结构管理

## 目录结构规范

```
rt-ee/
├── lab/                    # 可独立运行的 Rust 代码
│   ├── hello/
│   │   ├── Cargo.toml
│   │   └── src/main.rs
│   ├── ownership/
│   ├── borrowing/
│   └── ...
├── docs/                    # 文档（每个模块一个目录）
│   ├── README.md
│   ├── hello/
│   │   └── hello.md
│   ├── ownership/
│   │   └── ownership.md
│   ├── borrowing/
│   │   └── borrowing.md
│   └── ...
└── README.md
```

## 核心约束

| 约束 | 说明 |
|------|------|
| lab/{module}/ | 每个模块独立的 Cargo.toml + src/main.rs |
| docs/{module}/ | 每个模块一个目录，目录内必须有 {module}.md |
| 模块名 | 只用小写字母和下划线 |

## 创建新模块流程

当用户要求添加 Rust 功能验证时：

### 1. 创建 lab 目录
```bash
mkdir -p lab/{module}/src
```

### 2. 创建 lab/{module}/Cargo.toml
```toml
[package]
name = "{module}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{module}"
path = "src/main.rs"
```

### 3. 创建 lab/{module}/src/main.rs
```rust
fn main() {
    println!("Hello from {module}!");
}
```

### 4. 创建 docs/{module}/{module}.md
```markdown
# {Module} 模块

## 概念说明

## 代码示例

## 运行方式

```bash
cargo run -p {module}
```
```

### 5. 更新 docs/README.md
在列表中添加新模块链接。

## 坑点警示

| 错误操作 | 实际后果 | 正确做法 |
|---------|---------|---------|
| docs 下直接放 md 文件 | 违反规范，不便扩展 | 必须 `docs/{module}/{module}.md` |
| 模块名用大写 | cargo 报错 | 只用小写字母和下划线 |
| docs 目录不同步更新 | 代码和文档脱节 | 创建模块时必须同步创建 docs |
