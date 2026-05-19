# 迭代器与闭包

## 概念

- 迭代器: 遍历序列元素
- 闭包: 匿名函数，可捕获环境

## 代码

```rust
fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let sum: i32 = v.iter().sum();
    println!("Sum: {}", sum);

    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    println!("Doubled: {:?}", doubled);

    let x = 4;
    let equal_to_x = |z| z == x;
    println!("{}", equal_to_x(4));
}
```

## 运行

```bash
cargo run -p iterators_closures
```
