# 错误处理 (Error Handling)

## 概念

使用 `Result` 和 `?` 运算符进行可恢复错误的处理。

## 代码

```rust
use std::fs::File;
use std::io::{self, Read};

fn read_file_contents(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() {
    match read_file_contents("Cargo.toml") {
        Ok(contents) => println!("{}", contents),
        Err(e) => println!("Error: {}", e),
    }
}
```

## 运行

```bash
cargo run -p error_handling
```
