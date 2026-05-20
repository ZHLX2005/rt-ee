# 迭代器与闭包

## 设计背景与问题域

### 核心问题：如何优雅地处理序列？

处理集合（数组、向量、列表等）是编程中的常见任务。传统方式：

```c
// C 风格：手动索引
for (int i = 0; i < len; i++) {
    printf("%d\n", arr[i]);
}
```

**问题**：
- 手动管理索引容易出错（越界）
- 难以并行化
- 无法链式操作

**Rust 的方案**：迭代器 + 适配器 + 消费器

---

## 抽象设计分析

### 迭代器模式

```rust
let v = vec![1, 2, 3, 4, 5];

// iter() 返回迭代器
let iter = v.iter();

// next() 是迭代器的核心方法
assert_eq!(iter.next(), Some(&1));
assert_eq!(iter.next(), Some(&2));
assert_eq!(iter.next(), Some(&3));
assert_eq!(iter.next(), None);
```

**迭代器是一种对象，它知道如何按顺序产生序列中的元素。**

### 迭代器适配器

适配器将迭代器转换为另一种迭代器，**不消费原始数据**：

```rust
let v = vec![1, 2, 3, 4, 5];

// map 是适配器：转换每个元素
let doubled: Vec<_> = v.iter().map(|x| x * 2).collect();
println!("{:?}", doubled); // [2, 4, 6, 8, 10]

// filter 是适配器：过滤元素
let evens: Vec<_> = v.iter().filter(|x| *x % 2 == 0).collect();
println!("{:?}", evens); // [2, 4]
```

### 迭代器消费器

消费器消耗迭代器并产生最终结果：

```rust
let v = vec![1, 2, 3, 4, 5];

// sum 是消费器：产生单一值
let sum: i32 = v.iter().sum();
println!("{}", sum); // 15

// collect 是消费器：产生集合
let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
```

---

## 核心规则

### Iterator trait

```rust
pub trait Iterator {
    type Item; // 关联类型：每个元素的类型

    fn next(&mut self) -> Option<Self::Item>;

    // 默认实现了很多适配器方法
}
```

### 三种迭代器借用模式

```rust
let v = vec![1, 2, 3];

v.iter()     // &T - 不可变借用
v.iter_mut() // &mut T - 可变借用
v.into_iter() // T - 获取所有权
```

### 链式调用

```rust
let result: i32 = vec![1, 2, 3, 4, 5]
    .iter()              // 获取迭代器
    .filter(|x| *x % 2 == 0)  // 过滤偶数
    .map(|x| x * x)     // 平方
    .sum();              // 求和

println!("{}", result); // 4 + 16 = 20
```

---

## 代码示例（带设计意图注释）

### 示例 1：基本迭代器

```rust
// 设计意图：迭代器是惰性的，只有消费时才执行
// 对比：Java 的 Stream 也是惰性的，但语法更复杂

fn main() {
    let v = vec![1, 2, 3, 4, 5];

    // iter() - 不可变借用
    for val in v.iter() {
        println!("{}", val);
    }
    println!("{:?}", v); // v 仍然有效

    // into_iter() - 获取所有权
    let v2 = vec![1, 2, 3];
    for val in v2.into_iter() {
        println!("{}", val);
    }
    // println!("{:?}", v2); // 错误！v2 已被消费
}
```

### 示例 2：迭代器适配器链

```rust
// 设计意图：链式调用让数据转换更表达力
// 对比：Java Stream API 类似，但 Rust 更简洁

fn main() {
    let numbers = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];

    // 过滤偶数，平方，只取前3个
    let result: Vec<i32> = numbers
        .iter()
        .filter(|&&x| x % 2 == 0)  // 过滤偶数
        .map(|x| x * x)            // 平方
        .take(3)                   // 只取前3个
        .collect();

    println!("{:?}", result); // [4, 16, 36]
}
```

### 示例 3：闭包捕获环境

```rust
// 设计意图：闭包可以捕获定义它的环境中的变量
// 对比：Java 的 lambda 只能捕获 effectively final 变量

fn main() {
    let x = 4;

    // 闭包捕获 x
    let equal_to_x = |z| z == x;
    println!("{}", equal_to_x(4)); // true
    println!("{}", equal_to_x(5)); // false

    // x 在闭包定义时捕获，之后 x 的变化不影响闭包
    let y = 10;
    let add_y = |z| z + y;
    println!("{}", add_y(5)); // 15
}
```

### 示例 4：闭包与迭代器

```rust
// 设计意图：闭包作为迭代器方法的参数，非常强大

struct Person {
    name: String,
    age: u32,
}

fn main() {
    let people = vec![
        Person { name: String::from("Alice"), age: 30 },
        Person { name: String::from("Bob"), age: 25 },
        Person { name: String::from("Charlie"), age: 35 },
    ];

    // 过滤年龄 >= 30，提取名字
    let names: Vec<String> = people
        .iter()
        .filter(|p| p.age >= 30)
        .map(|p| p.name.clone())
        .collect();

    println!("{:?}", names); // ["Alice", "Charlie"]
}
```

### 示例 5：自定义迭代器

```rust
// 设计意图：实现 Iterator trait 来创建自定义迭代器
// 这是 Rust 零成本抽象的体现

struct Counter {
    count: u32,
    max: u32,
}

impl Counter {
    fn new(max: u32) -> Counter {
        Counter { count: 0, max }
    }
}

impl Iterator for Counter {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.max {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

fn main() {
    let counter = Counter::new(5);

    // 使用自定义迭代器
    let sum: u32 = counter
        .filter(|x| x % 2 == 0)  // 过滤偶数
        .map(|x| x * x)           // 平方
        .sum();

    println!("{}", sum); // 4 + 16 = 20
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust 迭代器 | Java Stream | Go |
|------|------------|-----------|-----|
| **惰性求值** | 是 | 是 | 否（直接遍历） |
| **链式调用** | 是 | 是 | 否 |
| **性能** | 零开销（编译器优化） | 有一定开销 | 高性能 |
| **语法** | 简洁 | 较复杂 | 简单直接 |

### Java Stream

```java
List<Integer> result = numbers.stream()
    .filter(x -> x % 2 == 0)
    .map(x -> x * x)
    .limit(3)
    .collect(Collectors.toList());
```

**问题**：
- 语法冗长
- 内部迭代 vs 外部迭代的权衡
- 不是真正的惰性（某些操作触发执行）

### Go

```go
// Go 没有迭代器适配器，直接用 for range
for i, v := range numbers {
    if v % 2 == 0 {
        fmt.Println(v * v)
    }
}
```

**问题**：
- 没有链式调用
- 难以组合多个操作
- 无法并行化

---

## 闭包的深入分析

### Fn, FnMut, FnOnce

```rust
// Fn: 可以多次调用，不修改捕获的变量
let x = 2;
let square = |z| z * x; // Fn
println!("{}", square(3)); // 6

// FnMut: 修改捕获的变量
let mut sum = 0;
let mut add_to_sum = |z| sum += z; // FnMut
add_to_sum(5);
println!("{}", sum); // 5

// FnOnce: 消费捕获的变量
let s = String::from("hello");
let consume = || s; // FnOnce，s 被移动
// println!("{}", s); // 错误！s 已被移动
```

### move 闭包

```rust
// move 强制闭包获取变量的所有权
let data = vec![1, 2, 3];

let closure = move || {
    println!("{:?}", data);
    // data 被移动到闭包中
};

// println!("{:?}", data); // 错误！data 已被移动

closure(); // [1, 2, 3]
```

---

## 设计哲学

### 零成本抽象

Rust 的迭代器是**零成本抽象**的典型例子：

```rust
// 这个链式调用会被编译器优化为手写的循环
let result: i32 = (0..1000)
    .filter(|x| x % 2 == 0)
    .map(|x| x * x)
    .sum();
```

**编译器优化**：
1. 迭代器适配器会被内联
2. 循环会被合并
3. 最终生成接近手写的高效机器码

### 内部迭代 vs 外部迭代

| 类型 | 说明 | 示例 |
|------|------|------|
| 外部迭代 | 程序员控制循环 | for i in 0..n |
| 内部迭代 | 迭代器控制循环 | iter.map().filter() |

Rust 主要使用外部迭代（for 循环），但允许内部迭代（通过迭代器方法）。

---

## 总结

| 概念 | 说明 |
|------|------|
| Iterator trait | next() 方法产生序列元素 |
| 适配器 | map, filter, take 等，转换迭代器 |
| 消费器 | sum, collect, fold 等，消耗迭代器 |
| 闭包 | 匿名函数，可以捕获环境变量 |
| Fn/FnMut/FnOnce | 闭包实现的三个 trait |

**核心洞察**：Rust 的迭代器是零成本抽象——迭代器适配器在编译时被优化为高效机器码，接近手写的循环性能。
