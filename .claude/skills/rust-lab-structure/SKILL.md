---
name: rust-lab-structure
description: 当用户要求创建rust学习目录结构、添加新的rust功能验证模块、或需要管理lab和docs时触发
---

# Rust Lab 目录结构管理

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

## Step 2: 创建 lab/{module}（如有代码）

```bash
mkdir -p lab/{module}/src
```

## Step 3: 创建 Cargo.toml

```toml
[package]
name = "{module}"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "{module}"
path = "src/main.rs"
```

## Step 4: 创建 src/main.rs（如有代码）

根据用户需求编写示例代码，代码要能独立运行。

## Step 5: 创建 docs/{module}/{module}.md

**重要**：无论是否有代码，知识性问题也必须创建文档！

```markdown
# {Module} 模块

## 概念说明

## 关键规则

## 代码示例（可选）

## 运行（如果有代码）

```bash
cargo run -p {module}
```
```

## Step 6: 更新 docs/README.md

在目录列表中添加新模块链接。

---

# 场景二：知识问答记录

当用户提问 Rust 知识点时（如 rustc vs cargo、所有权规则等）：

## 执行步骤

1. **确定模块名** → 如 `rustc_vs_cargo`
2. **创建 docs/{module}/{module}.md** → 整理知识内容
3. **更新 docs/README.md** → 添加链接

## 文档格式

```markdown
# rustc 与 cargo 的区别

## rustc

## cargo

## 关系

## 常见用法
```

---

# 示例对话

**用户**: "rustc和cargo的区别是什么"

**AI 执行**:
1. 模块名: `rustc_vs_cargo`
2. `mkdir -p docs/rustc_vs_cargo`
3. 创建 `docs/rustc_vs_cargo/rustc_vs_cargo.md` 整理知识点
4. 更新 `docs/README.md`

---

**用户**: "我想学习 rust 的并发编程"

**AI 执行**:
1. 模块名: `concurrency`
2. `mkdir -p lab/concurrency/src`
3. 创建 `lab/concurrency/Cargo.toml`
4. 创建 `lab/concurrency/src/main.rs` (包含 Mutex/Arc 示例)
5. 创建 `docs/concurrency/concurrency.md`
6. 更新 `docs/README.md`

---

# 坑点警示

| 错误操作 | 实际后果 | 正确做法 |
|---------|---------|---------|
| docs 直接放 md 文件 | 违反规范 | 必须 `docs/{module}/{module}.md` |
| 知识问答没记录文档 | 知识点丢失 | 所有用户提问都必须记录到 docs |
| lab 没有独立 Cargo.toml | 模块无法单独运行 | 每个 lab/{module} 需独立 Cargo.toml |
| 忘记同步创建 docs | 代码和文档脱节 | lab 和 docs 必须同步创建 |
| 模块名用大写 | cargo 报错 | 只用小写字母和下划线 |
