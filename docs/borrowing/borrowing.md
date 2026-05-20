# 借用 (Borrowing)

## 设计背景与问题域

### 核心问题：如何使用值但不获取所有权？

所有权系统解决了"谁来释放资源"的问题，但有时候我们只是想**使用**一个值，而不是**拥有**它。

**所有权转移的问题**：

```rust
fn calculate_length(s: String) -> usize {
    s.len()
} // s 在这里被 drop

fn main() {
    let text = String::from("hello");
    let len = calculate_length(text);
    println!("{}", text); // 错误！text 的所有权已转移
}
```

**为什么 Java/Go 没有这个问题？**

- Java：所有对象都是引用传递，原对象仍然有效
- Go：函数参数是值拷贝，但对于引用类型（slice, map, channel），传递的是引用的拷贝，原变量仍然有效

**Rust 的方案**：引入**借用（Borrowing）**——允许使用值但不获取所有权。

---

## 抽象设计分析

### 借用是临时使用权的授予

借用类似于：
- **租房子**：你可以使用房子一段时间，但房子不属于你
- **图书馆借书**：你可以阅读，但书的所有权仍然是图书馆的

```rust
let s = String::from("hello");

let len = calculate_length(&s); // &s 表示"借用 s"
                                // s 的所有权仍然属于 main 函数
println!("{}", s); // OK！s 仍然有效
```

### &T vs &mut T

| 类型 | 含义 | 规则 |
|------|------|------|
| `&T` | 不可变借用 | 可以同时有多个 |
| `&mut T` | 可变借用 | 只能有一个，且不能与其他借用共存 |

### 借用检查器的规则

```
借用规则：
1. 任意数量的不可变借用（&T）可以同时存在
2. 只能有一个可变借用（&mut T）
3. 可变借用和不可变借用不能同时存在
```

**为什么这样设计？**

这三条规则确保了**数据竞争不可能**：
- 多个线程同时读取 → OK
- 一个线程写入，其他线程读取 → OK（读写锁）
- 多个线程同时写入 → 不允许

---

## 核心规则

### 借用规则详解

```rust
let mut s = String::from("hello");

// 不可变借用：可以多个
let r1 = &s;
let r2 = &s;
let r3 = &s;
println!("{} {} {}", r1, r2, r3); // OK

// r1, r2, r3 在这里之后不再使用

// 可变借用：只能有一个
let r4 = &mut s; // OK，因为之前的借用都已结束
println!("{}", r4);
```

### NLL（Non-Lexical Lifetimes）

**早期 Rust 的问题**：

```rust
let mut s = String::from("hello");
let r1 = &s;
let r2 = &s;
println!("{}", r1); // 以前必须在 println! 之后才能创建 &mut s
let r3 = &mut s; // 现代 Rust：只要 r1, r2 不再使用，就可以
```

NLL 是 Rust 2018 引入的改进：**借用检查器只追踪引用实际被使用的范围**，而不是引用的完整作用域。

---

## 代码示例（带设计意图注释）

### 示例 1：基本借用

```rust
// 设计意图：借用允许函数使用值但不获取所有权
// 对比：如果不用借用，函数会转移所有权
fn calculate_length(s: &String) -> usize {
    s.len()
    // s 是借用，不影响原值的所有权
    // 函数结束时 s 被归还，借用到此结束
} // 原始的 text 仍然有效

fn main() {
    let text = String::from("hello");
    let len = calculate_length(&text); // & 表示借用
    println!("'{}' 的长度是 {}", text, len); // text 仍然有效
}
```

### 示例 2：可变借用

```rust
// 设计意图：可变借用允许修改借用的值
// 但修改期间不能有其他借用（防止数据竞争）
fn append_world(s: &mut String) {
    s.push_str(", world");
}

fn main() {
    let mut text = String::from("hello");
    append_world(&mut text);
    println!("{}", text); // "hello, world"
}
```

### 示例 3：数据竞争防止

```rust
// 设计意图：借用规则在编译时就防止了数据竞争
// 这段代码会编译失败，而不是等到运行时才出问题

fn main() {
    let mut data = vec![1, 2, 3];

    let first = &data;       // 不可变借用
    // data.push(4);         // 错误！可变借用与不可变借用冲突
    println!("{}", first[0]);
}

// 编译器输出：
// error[E0502]: cannot borrow `data` as mutable because it is also borrowed as immutable
```

### 示例 4：结构体方法中的借用

```rust
// 设计意图：方法通常需要借用 self
// 这样调用者仍然持有所有权

struct Counter {
    count: i32,
}

impl Counter {
    fn get_count(&self) -> i32 {
        self.count
    }

    fn increment(&mut self) {
        self.count += 1;
    }
}

fn main() {
    let mut counter = Counter { count: 0 };
    println!("{}", counter.get_count()); // &self 借用
    counter.increment(); // &mut self 借用
    println!("{}", counter.get_count());
}
```

### 示例 5：借用作为函数参数

```rust
// 设计意图：借用参数避免了所有权的转移
// 调用者仍然持有值的所有权

fn find_longest<'a>(slice: &'a [i32]) -> i32 {
    slice.iter().max().copied().unwrap_or(0)
}

fn main() {
    let numbers = vec![1, 5, 3, 9, 2];
    let longest = find_longest(&numbers); // 借用 numbers
    println!("最长: {}", longest);
    println!("原始数据: {:?}", numbers); // numbers 仍然有效
}
```

---

## 与 Java/Go 的深度对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **函数参数传递** | 值拷贝 or 借用 | 引用传递 | 值拷贝（引用类型是引用拷贝） |
| **可变参数** | 必须 `&mut` | 可以在方法内修改 | 可以在方法内修改 |
| **数据竞争** | 编译时检查 | 运行时检查 | 运行时检查 |
| **空指针** | Option 类型 | NPE | nil |

### Java 的"一切皆引用"

```java
void foo(String s) {
    // s 是引用，指向原对象
    // 原对象仍然有效
}

String text = "hello";
foo(text);
System.out.println(text); // OK
```

**Rust 的等价实现**：

```rust
fn foo(s: &String) {
    // s 是借用，原值仍然有效
}

let text = String::from("hello");
foo(&text);
println!("{}", text); // OK
```

### Go 的值拷贝

```go
func foo(s string) {
    // s 是值的拷贝
    // 原变量仍然有效
}

func bar(s []int) {
    // s 是 slice header 的拷贝
    // 底层数组仍然是共享的
}

s := []int{1, 2, 3}
bar(s) // s 仍然有效，但底层数组共享
```

---

## 常见错误与正确模式

### 错误 1：可变借用和不可变借用同时存在

```rust
let mut s = String::from("hello");
let r1 = &s;      // 不可变借用
let r2 = &mut s;  // 错误！可变借用与不可变借用冲突
```

**正确模式**：分开作用域

```rust
let mut s = String::from("hello");
{
    let r1 = &s;
    println!("{}", r1);
} // r1 结束
let r2 = &mut s; // OK
```

### 错误 2：在借用期间修改值

```rust
let mut v = vec![1, 2, 3];
let first = &v[0]; // 不可变借用
v.push(4);        // 错误！可能导致 v 重新分配内存，first 悬空
println!("{}", first);
```

### 错误 3：返回借用而不是值

```rust
// 错误！
fn dangle() -> &String {
    let s = String::from("hello");
    &s // 返回对局部变量的引用
}
```

**正确模式**：返回值而不是借用

```rust
// 正确：返回值（所有权转移）
fn get_string() -> String {
    let s = String::from("hello");
    s // 移动返回值
}
```

---

## 设计哲学

### 为什么 Rust 选择借用而不是自动引用？

**Rust 的设计原则：显式优于隐式**

```rust
// Java：隐式引用
void foo(String s) { } // s 是引用

// Rust：显式借用
fn foo(s: &String) { } // 必须写 & 表示借用
```

**为什么 Rust 不默认使用引用？**

1. **所有权语义**：如果函数参数默认是引用，调用者可能不清楚所有权是否转移
2. **性能**：有些情况下需要 Copy（栈拷贝），而不是引用
3. **明确性**：`&` 和 `&mut` 让代码意图一目了然

### 借用检查器的实现

借用检查器通过 **MIR（Mid-level IR）** 进行分析：

```
源代码
    ↓
MIR（借用检查的中间表示）
    ↓
区域（Region）分析
    ↓
NLL（Non-Lexical Lifetimes）检查
    ↓
确定借用是否有效
```

---

## 总结

| 概念 | 说明 |
|------|------|
| `&T` | 不可变借用，可以有多个 |
| `&mut T` | 可变借用，只能有一个 |
| 借用规则 | 不可变借用可共存，可变借用独占 |
| NLL | 非词法生命周期，只追踪实际使用范围 |
| 设计意图 | 使用值但不获取所有权，避免数据竞争 |

**核心洞察**：Rust 的借用系统是"借用检查器"的核心，它确保了在编译时就排除数据竞争，而不是等到运行时才发现问题。
