# 模块 (Modules)

## 概念

组织代码结构，控制作用域和可见性。

## 代码

```rust
mod outer {
    pub fn public_function() {
        println!("Public function");
    }

    pub mod inner {
        pub fn inner_function() {
            println!("Inner function");
        }
    }
}

fn main() {
    outer::public_function();
    outer::inner::inner_function();
}
```

## 运行

```bash
cargo run -p modules
```
