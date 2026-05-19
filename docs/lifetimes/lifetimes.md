# 生命周期 (Lifetimes)

## 概念

确保引用始终有效，防止悬垂引用。

## 代码

```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() { x } else { y }
}

fn main() {
    let s1 = String::from("long string");
    let result;
    {
        let s2 = String::from("xyz");
        result = longest(s1.as_str(), s2.as_str());
        println!("Longest: {}", result);
    }
}
```

## 运行

```bash
cargo run -p lifetimes
```
