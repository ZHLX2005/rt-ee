# Option 与 Result

## 概念

- `Option<T>`: 表示可能存在或不存在的值
- `Result<T, E>`: 表示可能成功或失败的操作

## 代码

```rust
fn main() {
    let some_value: Option<i32> = Some(5);
    match some_value {
        Some(v) => println!("Got value: {}", v),
        None => println!("No value"),
    }

    let result: Result<i32, &str> = Ok(42);
    match result {
        Ok(v) => println!("Success: {}", v),
        Err(e) => println!("Error: {}", e),
    }
}
```

## 运行

```bash
cargo run -p option_result
```
