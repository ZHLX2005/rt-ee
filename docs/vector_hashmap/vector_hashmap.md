# Vector 与 HashMap

## 概念

- `Vec<T>`: 可增长的数组
- `HashMap<K, V>`: 键值对集合

## 代码

```rust
use std::collections::HashMap;

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    println!("v = {:?}", v);
    println!("First: {:?}", v.first());

    let mut scores = HashMap::new();
    scores.insert(String::from("Blue"), 10);
    scores.insert(String::from("Yellow"), 50);
    println!("{:?}", scores);
}
```

## 运行

```bash
cargo run -p vector_hashmap
```
