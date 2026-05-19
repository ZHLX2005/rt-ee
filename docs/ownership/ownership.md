# 所有权 (Ownership)

## 概念

Rust 的核心特性：每个值有唯一的 owner，当 owner 离开作用域，值被 drop。

## 规则

1. 每个值有一个 owner
2. 同一时间只有一个 owner
3. 当 owner 离开作用域，值被 drop

## 代码

```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // s1 is moved to s2

    println!("s2 = {}", s2);

    // Clone
    let s3 = s2.clone();
    println!("s2 = {}, s3 = {}", s2, s3);
}
```

## 运行

```bash
cargo run -p ownership
```
