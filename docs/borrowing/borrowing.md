# 借用 (Borrowing)

## 概念

通过引用借用值，而不获取所有权。

## 规则

- 可以有多个不可变引用 `&T`
- 只能有一个可变引用 `&mut T`
- 引用必须始终有效

## 代码

```rust
fn calculate_length(s: &String) -> usize {
    s.len()
}

fn main() {
    let s = String::from("hello");
    let len = calculate_length(&s);
    println!("Length of '{}' is {}", s, len);
}
```

## 运行

```bash
cargo run -p borrowing
```
