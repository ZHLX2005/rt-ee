# 泛型与 Trait

## 概念

- 泛型: 创建可复用的代码，适用于多种类型
- Trait: 定义共享行为接口

## 代码

```rust
trait Summary {
    fn summarize(&self) -> String;
}

struct Article {
    title: String,
    author: String,
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{} by {}", self.title, self.author)
    }
}

fn main() {
    let article = Article {
        title: String::from("Rust Programming"),
        author: String::from("Author"),
    };
    println!("{}", article.summarize());
}
```

## 运行

```bash
cargo run -p generics_traits
```
