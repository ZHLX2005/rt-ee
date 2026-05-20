# Vector 与 HashMap

## 设计背景与问题域

### 核心问题：如何存储和管理集合数据？

编程中需要存储多个值：
- 列表：有序、可重复
- 映射：键值对，快速查找

**传统语言的方案**：
- C：静态数组 / 手动链表
- Java：ArrayList / HashMap
- Go：slice / map

**Rust 的方案**：`Vec<T>` + `HashMap<K, V>`

---

## Vec<T>：动态数组

### 核心概念

```rust
let mut v = Vec::new();    // 创建空 Vec
v.push(1);                  // 添加元素
v.push(2);
v.push(3);

let v = vec![1, 2, 3];    // 宏创建
```

### Vec 的内存布局

```rust
let v = vec![1, 2, 3, 4, 5];
// Vec 包含：
// - ptr: 指向堆上的数据
// - len: 当前元素数量
// - capacity: 已分配的容量
```

**关键特性**：
- 容量不足时自动重新分配（2x 增长）
- 重新分配会移动所有元素
- `push` 可能触发重新分配

---

## HashMap<K, V>：键值对集合

### 核心概念

```rust
use std::collections::HashMap;

let mut scores = HashMap::new();
scores.insert(String::from("Blue"), 10);
scores.insert(String::from("Yellow"), 50);
```

### HashMap 的查找

```rust
// get 返回 Option<&V>
match scores.get("Blue") {
    Some(value) => println!("Blue team: {}", value),
    None => println!("Blue team not found"),
}

// 更简洁：unwrap_or
let score = scores.get("Blue").copied().unwrap_or(0);

// 或者用 entry API
scores.entry(String::from("Red")).or_insert(30);
```

---

## 代码示例（带设计意图注释）

### 示例 1：Vec 基本操作

```rust
// 设计意图：Vec 是 Rust 最常用的集合类型
// 对比：类似于 Java 的 ArrayList，但更安全

fn main() {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);

    // 索引访问
    println!("First: {}", v[0]);       // 直接索引，越界 panic

    // get 方法返回 Option
    println!("First: {:?}", v.get(0));  // Some(&1)
    println!("Tenth: {:?}", v.get(9)); // None

    // 遍历
    for i in &v {
        println!("{}", i);
    }

    // 容量管理
    println!("Len: {}, Capacity: {}", v.len(), v.capacity());
}
```

### 示例 2：Vec 与所有权

```rust
// 设计意图：Vec 中的元素遵循所有权规则
// 对比：Java 的 ArrayList 存储引用，不涉及所有权

fn main() {
    let v = vec![String::from("hello"), String::from("world")];

    // 移动元素出 Vec
    let s = v.into_iter().next().unwrap();
    println!("{}", s); // hello

    // v 已被消费，不能再使用
    // println!("{:?}", v); // 编译错误！
}
```

### 示例 3：HashMap 基本操作

```rust
// 设计意图：HashMap 提供 O(1) 平均查找
// 对比：类似于 Java 的 HashMap 或 Go 的 map

use std::collections::HashMap;

fn main() {
    let mut scores = HashMap::new();

    // 插入
    scores.insert(String::from("Blue"), 10);
    scores.insert(String::from("Yellow"), 50);

    // 查找
    let team_name = String::from("Blue");
    let score = scores.get(&team_name);
    println!("Blue team score: {:?}", score);

    // 更新
    scores.insert(String::from("Blue"), 25); // 覆盖原有值

    // entry API：只在键不存在时插入
    scores.entry(String::from("Green")).or_insert(100);

    println!("{:?}", scores);
}
```

### 示例 4：HashMap 与迭代

```rust
use std::collections::HashMap;

fn main() {
    let teams = vec![
        String::from("Blue"),
        String::from("Yellow"),
        String::from("Blue"),
    ];

    // 统计每个团队出现的次数
    let mut count = HashMap::new();

    for team in &teams {
        // entry 返回 Entry，可以检查是否存在
        let num = count.entry(team).or_insert(0);
        *num += 1;
    }

    println!("{:?}", count); // {"Blue": 2, "Yellow": 1}
}
```

### 示例 5：Vec 性能注意事项

```rust
// 设计意图：Vec 的 push 可能触发重新分配
// 预分配容量可以避免这个问题

fn main() {
    // 方式 1：预先分配容量
    let mut v = Vec::with_capacity(1000);
    for i in 0..1000 {
        v.push(i);
    }
    println!("Capacity after pre-allocation: {}", v.capacity());

    // 方式 2：逐个 push（可能多次重新分配）
    let mut v2 = Vec::new();
    for i in 0..1000 {
        v2.push(i);
    }
    println!("Capacity after push: {}", v2.capacity());
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust Vec | Java ArrayList | Go slice |
|------|----------|---------------|----------|
| **增长策略** | 2x | 1.5x | 2x |
| **类型安全** | 泛型 T | 泛型 T | 泛型（interface{}） |
| **所有权** | 移动语义 | 引用语义 | 引用语义 |
| **访问方式** | 索引/get | 索引/get | 索引 |

| 维度 | Rust HashMap | Java HashMap | Go map |
|------|-------------|--------------|--------|
| **查找复杂度** | O(1) 平均 | O(1) 平均 | O(1) 平均 |
| **键要求** | Eq + Hash trait | equals + hashCode | 必须可比较 |
| **并发安全** | 标准库无（用 dashmap） | ConcurrentHashMap | 原生并发安全 |

### Java 的 ArrayList

```java
List<String> list = new ArrayList<>();
list.add("hello");
list.add("world");
list.get(0); // "hello"
```

**问题**：
- 可以存储 null
- 泛型有类型擦除

### Go 的 slice

```go
slice := []int{1, 2, 3}
slice = append(slice, 4)
```

**问题**：
- 可以 append nil slice
- 容量增长不透明

---

## 设计哲学

### Vec 的零成本抽象

```rust
let v: Vec<i32> = (0..1000).collect();
```

**编译器优化**：
- `collect()` 可以直接分配正确大小的内存
- 迭代器可以被融合，避免中间分配
- SIMD 优化可能应用于元素操作

### HashMap 的 trait bounds

```rust
use std::collections::HashMap;

fn count_words(text: &str) -> HashMap<&str, i32> {
    let mut map = HashMap::new();
    for word in text.split_whitespace() {
        *map.entry(word).or_insert(0) += 1;
    }
    map
}
```

**要求**：`K` 必须实现 `Eq + Hash trait`

---

## 总结

| 类型 | 说明 |
|------|------|
| Vec<T> | 动态数组，支持 push/pop/index |
| HashMap<K, V> | 键值对集合，O(1) 查找 |
| HashSet<T> | 只有键的集合（基于 HashMap） |
| VecDeque<T> | 双端队列 |

**核心洞察**：Rust 的集合类型通过 trait bounds 提供类型安全，同时保持零成本抽象的性能。
