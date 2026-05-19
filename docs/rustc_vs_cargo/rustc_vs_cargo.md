# rustc 与 cargo 的区别

## rustc

Rust 编译器，编译 Rust 源代码生成可执行文件或库。

```bash
rustc main.rs -o main
```

## cargo

Rust 的包管理器和构建工具。

- 管理依赖
- 编译项目
- 运行测试
- 生成文档

## 关系

- `cargo` 底层调用 `rustc` 进行编译
- `cargo` 是开发者的主要入口
- `rustc` 在需要直接编译单个文件时使用

## 常见用法

| 命令 | 说明 |
|------|------|
| `cargo new project` | 创建新项目 |
| `cargo build` | 编译项目 |
| `cargo run` | 运行项目 |
| `cargo test` | 运行测试 |
| `cargo doc` | 生成文档 |
| `cargo add crate` | 添加依赖 |
