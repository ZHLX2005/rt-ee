# Trait 与父类（继承）的区别

## 核心结论

**Trait 不是父类。**

Trait 是**能力契约**（can-do），父类是**类型层次**（is-a）。这是两个根本不同的概念。

---

## 设计背景与问题域

### 为什么会有这个误解？

因为 Java 的 Interface 和 Rust 的 Trait 听起来很像，而 Java 的 Interface 常被用来模拟"多重继承"，所以容易产生"Trait = 父类/接口"的误解。

但 Trait 和父类（抽象类）有本质区别：

| 维度 | Rust Trait | Java 抽象类（父类） |
|------|------------|---------------------|
| **核心概念** | 能力/行为契约 | is-a 继承关系 |
| **字段** | 不能存储状态 | 可以有字段 |
| **构造函数** | 没有 | 可以有构造方法 |
| **多实现** | 可以 impl 任意多个 trait | 只能单继承 |
| **类型层次** | 组合优于继承 | 继承树 |
| **状态转移** | 无（借用/移动语义） | 子类拥有父类状态 |

---

## 抽象设计分析

### Trait 是"能做什么"，不是"是什么"

```rust
// 父类思维（Java）
class Dog extends Animal {
    // Dog is an Animal
}

// Trait 思维（Rust）
impl Speakable for Dog { }
impl Cloneable for Dog { }
impl Iterator for Dog { }
// Dog can speak, can be cloned, can be iterated
```

**设计意图**：Rust 选择 Trait 组合而非继承，是因为继承带来诸多问题：
- **脆弱基类问题**：父类改动可能破坏子类
- **菱形继承**：C++ 的噩梦
- **is-a 滥用**：明明是 has-a 能力，非要说 is-a

### Trait 不继承状态

```rust
// Java 父类：可以定义字段，子类继承状态
class Animal {
    String name; // 子类自动拥有这个字段
}

// Rust Trait：不能定义字段，只有行为
trait Speakable {
    fn speak(&self); // 没有 name 字段！
}
```

**为什么这样设计？**
- 如果 trait 有字段，实现者如何初始化？
- 多个 trait 都有字段怎么办？
- Rust 选择让类型自己管理状态，trait 只定义行为契约

### 多 Trait 组合优于单继承

```rust
// Rust：一个类型可以实现多个 trait
struct MyStruct;

impl Serialize for MyStruct {}  // 序列化能力
impl Debug for MyStruct {}      // 调试能力
impl Clone for MyStruct {}     // 克隆能力

// 对比 Java：只能单继承
class MyClass extends Parent implements Serializable, Cloneable {
    // 但父类只能有一个
}
```

---

## 核心区别对比

### 字段对比

```java
// Java 抽象类可以有字段
abstract class Animal {
    private String name; // 子类继承这个字段
    public Animal(String name) { this.name = name; }
}
```

```rust
// Rust trait 不能有字段
trait Animal {
    // 错误！trait 不能定义字段
    // name: String;
}
```

### 构造函数对比

```java
// Java 抽象类可以有构造方法
abstract class Animal {
    public Animal(String name) { }
}
```

```rust
// Rust trait 没有构造函数
trait Animal {
    // 错误！trait 不能有构造方法
    // fn new(name: String) -> Self;
}
```

### 多实现对比

```rust
// Rust 可以为任意类型实现任意多个 trait
struct Dog;

impl Speakable for Dog {}
impl Cloneable for Dog {}
impl Serializable for Dog {}
impl Debug for Dog {}

// Java 只能单继承
class Dog extends Animal implements Speakable, Serializable {
    // 但 extends 只能有一个类
}
```

---

## Trait 更像 Java Interface

```java
// Java Interface（更像 Rust Trait）
interface Summary {
    String summarize();
    default String summarize_author() { return "(Unknown)"; } // Java 8+ 支持默认实现
}
```

```rust
// Rust Trait（类似 Java Interface）
trait Summary {
    fn summarize(&self) -> String;
    fn summarize_author(&self) -> String {
        String::from("(Unknown Author)") // 默认实现
    }
}
```

**关键区别**：Rust Trait 支持默认实现早于 Java（Java 8 才引入 default 方法），而且 Rust Trait 还支持关联类型，这是 Java Interface 不支持的。

---

## 为什么 Rust 不选择类继承？

### 继承的问题

1. **脆弱基类问题**
   ```java
   // 基类
   class Animal {
       void eat() { /* ... */ }
   }

   // 子类依赖 eat 的实现
   class Dog extends Animal { }

   // 基类修改可能导致子类破坏
   class Animal {
       void eat() { /* 修改了 */ }
   }
   ```

2. **菱形继承问题**
   ```cpp
   // C++ 菱形继承
   class A { int x; };
   class B : public A {};
   class C : public A {};
   class D : public B, public C { }; // x 有两份！
   ```

3. **is-a 滥用**
   ```java
   // 滥用继承：Stack extends ArrayList
   // Stack is NOT an ArrayList，Stack HAS an ArrayList
   class Stack<T> extends ArrayList<T> { } // 错误设计
   ```

### Rust 的方案：组合优于继承

```rust
// Rust：用组合代替继承
struct Stack<T> {
    items: Vec<T>, // 组合：Stack HAS a Vec
}

impl<T> Stack<T> {
    fn push(&mut self, item: T) { self.items.push(item); }
    fn pop(&mut self) -> Option<T> { self.items.pop(); }
}
```

---

## 总结

| 问题 | 回答 |
|------|------|
| Trait 是父类吗？ | **不是**。Trait 是能力契约，不是类型层次。 |
| Trait 是什么？ | Trait 定义"能做什么"，不定义"是什么"。 |
| Trait 像什么？ | 更像 Java Interface，但更强大（有关联类型、默认实现）。 |
| 什么时候用 Trait？ | 当你想表达"这个类型能做某事"时。 |
| 什么时候用继承？ | Rust 不推荐使用继承，用 Trait 组合代替。 |

---

## 运行

```bash
cargo run -p generics_traits
```

参考：`docs/generics_traits/generics_traits.md`
