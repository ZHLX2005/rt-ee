# Rust 基础语法：let、mut、？

## 设计背景与问题域

### 核心问题

这三个语法元素是 Rust 最基础的概念，但每个都有独特的设计意图：

| 语法 | 问题 | Rust 的答案 |
|------|------|------------|
| `let` | 如何声明变量？ | 声明并绑定，变量默认不可变 |
| `mut` | 如何让变量可变？ | 显式标记可变性 |
| `?` | 如何处理错误？ | 错误传播的语法糖 |

---

## let：变量绑定

### 核心概念

`let` 不是"赋值"，而是**绑定（Binding）**：

```rust
let x = 5; // 将值 5 绑定到变量 x
```

**为什么叫绑定？**

- 绑定意味着 x 指向值 5 的内存位置
- 之后 x = 10 不是"改变 x 的值"，而是"将 x 重新绑定到新的值"
- 值 5 仍然存在于内存中（直到离开作用域）

### 绑定 vs 赋值

```rust
// Java/C：赋值
int x = 5;
x = 10; // 改变 x 指向的内存位置的值

// Rust：绑定
let x = 5;
x = 10; // 错误！x 默认不可变
let mut x = 5;
x = 10; // OK，x 重新绑定到新值
```

**关键区别**：
- Rust 的 `let` 是**声明**，不是语句
- 绑定后的变量默认不可变
- 可变性需要显式标记

### 模式匹配

```rust
let (a, b) = (1, 2); // 解构绑定
let [first, second, ...] = [1, 2, 3, 4]; // 切片模式

// _ 忽略值
let (_, b, _) = (1, 2, 3); // b = 2
```

### 延迟初始化

```rust
let x; // 声明但不初始化
x = 5; // 之后赋值
```

---

## mut：可变性标记

### 核心概念

`mut` 不是类型的一部分，而是**可变性标记**：

```rust
let mut x = 5; // x 现在是可变的
x = 10; // OK
```

### mut 的设计意图

**为什么可变性需要显式标记？**

1. **明确性**：`mut` 让代码意图一目了然
2. **安全性**：编译器检查所有对可变数据的访问
3. **性能**：可变数据可能需要不同的内存布局

### mut vs Java/C

```java
// Java：默认可变
int x = 5;
x = 10; // OK，x 默认可变

// Rust：默认不可变
let x = 5;
x = 10; // 错误！
let mut x = 5;
x = 10; // OK，显式标记
```

### 可变引用的规则

```rust
let mut s = String::from("hello");

// 不可变引用
let r1 = &s;

// 可变引用 - 不能与不可变引用共存
let r2 = &mut s; // 错误！

// 正确：先结束不可变引用的使用
println!("{}", r1); // r1 最后使用点
let r3 = &mut s; // OK
```

---

## ?：错误传播运算符

### 核心概念

`?` 是错误传播的**语法糖**：

```rust
fn read_file(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?; // 失败直接返回
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}
```

### 等价展开

```rust
// 使用 ?
let mut file = File::open(path)?;

// 等价于
let mut file = match File::open(path) {
    Ok(f) => f,
    Err(e) => return Err(e.into()),
};
```

### ? 可以链式使用

```rust
fn parse_config(path: &str) -> Result<Config, Box<dyn Error>> {
    let contents = std::fs::read_to_string(path)?; // ?
    let config: Config = toml::from_str(&contents)?; // ?
    Ok(config)
}
```

### ? 与 Option

```rust
fn get_first_char(s: &str) -> Option<char> {
    s.chars().next() // 返回 Option
}

fn process(s: &str) -> Option<char> {
    let first = get_first_char(s)?; // ?
    Some(first.to_ascii_uppercase())
}
```

### ? 的限制

```rust
// ? 只能用于返回 Result/Option 的函数
fn foo() -> Result<i32, E> {
    let x = some_result()?; // OK
    let y = some_option()?;  // OK
}

// 不能用于返回其他类型的函数
fn bar() -> i32 {
    let x = some_result()?; // 错误！
}
```

---

## 三者结合的实际例子

### 典型 Rust 代码

```rust
use std::fs::File;
use std::io::{self, Read};

fn read_config(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;        // let + mut + ?
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;     // mut + ?
    Ok(contents)
}

fn main() {
    let result = read_config("config.toml");
    match result {
        Ok(contents) => println!("Config: {}", contents),
        Err(e) => eprintln!("Error: {}", e),
    }
}
```

---

## 与 Java/Go 的对比

| 语法 | Rust | Java | Go |
|------|------|------|-----|
| 变量声明 | `let x = 5` | `int x = 5` | `x := 5` |
| 不可变 | `let x = 5` | `final int x = 5` | `const x = 5` |
| 可变 | `let mut x = 5` | `int x = 5` | `x = 5` |
| 错误传播 | `?` | `throws` | 多返回值 |

---

## 设计哲学

### 显式优于隐式

```rust
// Java：隐式可变
x = 10;

// Rust：显式标记
let mut x = 5;
x = 10;
```

### 最小权限原则

```rust
// 默认不可变 - 最小权限
let x = 5;

// 如果需要可变，显式标记
let mut y = 5;
y = 10;
```

### 错误作为值

```rust
// 不抛异常，而是返回 Result
fn divide(a: f64, b: f64) -> Result<f64, &'static str> {
    if b == 0.0 {
        Err("division by zero")
    } else {
        Ok(a / b)
    }
}

// ? 让错误传播简洁
fn calculate() -> Result<f64, &'static str> {
    let result = divide(10.0, 0.0)?;
    Ok(result * 2.0)
}
```

---

## 总结

| 语法 | 含义 | 设计意图 |
|------|------|---------|
| `let` | 变量绑定 | 默认不可变，声明式 |
| `mut` | 可变性标记 | 显式标记，编译器检查 |
| `?` | 错误传播 | 语法糖，避免样板代码 |

**核心洞察**：
- `let` 是绑定，不是赋值，绑定后的变量默认不可变
- `mut` 是显式可变性标记，符合 Rust 的"显式优于隐式"原则
- `?` 是错误传播的语法糖，让错误处理更简洁
