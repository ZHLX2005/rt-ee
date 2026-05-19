# 结构体 (Structs)

## 概念

自定义数据类型，将多个相关值组合在一起。

## 代码

```rust
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }

    fn square(size: u32) -> Rectangle {
        Rectangle { width: size, height: size }
    }
}

fn main() {
    let rect = Rectangle { width: 30, height: 50 };
    println!("Area: {}", rect.area());

    let sq = Rectangle::square(10);
    println!("Square area: {}", sq.area());
}
```

## 运行

```bash
cargo run -p structs
```
