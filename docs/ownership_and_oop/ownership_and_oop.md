# 所有权与 OOP 设计模式：失效了吗？心智负担大吗？

## 设计背景与问题域

从 Java 转向 Rust 时，一个自然的焦虑是：**Rust 的所有权系统会不会让经典的 OOP 设计模式无法使用？**

这种担忧的根源在于：
1. **所有权转移**意味着变量赋值后原变量失效——这打破了"引用传递"的直觉
2. **借用检查器**限制了同一时间对数据的可变访问——这让"随时修改对象状态"变得困难
3. **没有 GC**意味着需要显式管理对象生命周期——这让"创建对象然后忘记"不再可行

Java 的 OOP 设计模式建立在以下假设上：
- 对象在堆上长期存在，多个引用可以无限期地指向它
- 对象状态可以随时被任何持有者修改
- GC 负责清理不再使用的对象

Rust 打破了这些假设。但这并不意味着设计模式"失效"了——而是需要**用 Rust 的方式重新实现**。

---

## 核心回答

> **所有权不会导致 OOP 设计模式失效。**
>
> Rust 通过智能指针系统（`Box`、`Rc`、`Arc`、`RefCell`、`Mutex` 等）提供了多种所有权语义，可以实现所有经典设计模式。
>
> **心智负担确实存在**，但这是"显式控制换取编译期安全保证"的权衡。

---

## Rust 的智能指针系统：OOP 的瑞士军刀

Rust 的所有权系统不是"只有 move 一种选择"。编译器提供了一套完整的**所有权语义组合**：

| 智能指针 | 所有权语义 | 可变性 | 线程安全 | 最接近的 Java 概念 |
|---------|-----------|--------|---------|------------------|
| `Box<T>` | 独占所有权 | 可变（`Box<T>`）或不可变 | 是（如果 T 是 Send）| `new` 创建的单一强引用 |
| `Rc<T>` | 共享所有权（引用计数）| 不可变 | 否（单线程）| `new` 创建 + 多引用 |
| `Rc<RefCell<T>>` | 共享所有权 + 内部可变 | 可变 | 否（单线程）| 没有 synchronized 的对象 |
| `Arc<T>` | 线程安全共享所有权 | 不可变 | 是 | `ConcurrentHashMap` 的值 |
| `Arc<Mutex<T>>` | 线程安全共享 + 互斥可变 | 可变 | 是 | `synchronized` 对象 |
| `Arc<RwLock<T>>` | 线程安全共享 + 读写锁 | 可变 | 是 | `ReadWriteLock` |

**关键洞察**：Java 默认提供的是最后一行（`Arc<Mutex<T>>`）的语义——线程安全、共享可变、GC 管理。Rust 让你**显式选择**所需的语义组合，而不是默认给你最贵的那个。

---

## 经典设计模式的 Rust 实现

### 工厂模式

**Java 方式**：
```java
Animal animal = AnimalFactory.create("dog"); // 返回堆对象的引用
animal.speak(); // 多个变量可以持有同一引用
```

**Rust 方式**：
```rust
fn create_animal(kind: AnimalType) -> Box<dyn Animal> {
    match kind {
        AnimalType::Dog => Box::new(Dog),
        AnimalType::Cat => Box::new(Cat),
    }
}

let animal = create_animal(AnimalType::Dog);
animal.speak();
// animal 离开作用域时自动 drop
```

**差异分析**：
- Java 返回引用，调用者不拥有对象，GC 管理生命周期
- Rust 返回 `Box`，调用者**拥有**对象，确定性析构
- 如果 Java 中需要"拥有"语义（确保工厂不再保留引用），没有任何机制保证这一点
- Rust 中如果需要共享语义，调用者可以将 `Box` 转为 `Rc` 或 `Arc`

### 观察者模式

**Java 方式**：
```java
Subject subject = new Subject();
Observer email = new EmailObserver();
subject.attach(email); // subject 持有 email 的引用
subject.attach(email); // 甚至可以重复附加同一个观察者
// 如果忘记 detach，可能导致内存泄漏
```

**Rust 方式**：
```rust
let mut subject = Subject::new();
let email: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(EmailObserver));

subject.attach(Rc::clone(&email));
// subject.attach(Rc::clone(&email)); // 如果重复附加，可以通过代码检查避免

subject.notify("new event");
```

**关键差异**：
- Rust 使用 `Rc`（引用计数）实现共享所有权，当最后一个引用消失时自动释放
- 使用 `Weak` 指针可以避免循环引用导致的内存泄漏（Java 需要开发者手动打破循环或依赖 GC 的弱引用）
- `RefCell` 在**运行时**检查借用规则，如果同时存在可变和不可变借用会 panic（相当于运行时的断言）

### 状态模式

**Java 方式**：
```java
class Post {
    private State state = new DraftState();

    public void publish() {
        if (state != State.DRAFT) throw new IllegalStateException();
        state = new PublishedState();
    }
}
// 运行时检查状态，可能在生产环境抛出异常
```

**Rust 方式（类型状态）**：
```rust
struct Post<State> { content: String, _state: PhantomData<State> }

impl Post<Draft> {
    fn request_review(self) -> Post<PendingReview> { ... }
}
impl Post<PendingReview> {
    fn approve(self) -> Post<Published> { ... }
}
impl Post<Published> {
    fn content(&self) -> &str { ... }
}

// let post = post.approve(); // 错误：Draft 不能直接 approve！
```

**Rust 的优势**：
- 非法状态转换在**编译期**就被阻止
- 不需要运行时检查，零开销
- 不需要测试"非法状态操作"的路径

### 策略模式

**Java 方式**：
```java
PaymentStrategy strategy = new CreditCardStrategy("1234");
ShoppingCart cart = new ShoppingCart(strategy);
cart.checkout(100.0);
```

**Rust 方式**：
```rust
let cart = ShoppingCart::with_strategy(
    Box::new(CreditCard { number: "1234".to_string() })
);
cart.checkout(100.0);
```

**等价性**：两者几乎相同。Rust 的 `Box<dyn PaymentStrategy>` 等价于 Java 的接口引用。

---

## 心智负担：大，但有回报

### 为什么心智负担大？

**Java 程序员需要重新思考的问题**：

| 直觉 | Java 现实 | Rust 现实 |
|------|----------|----------|
| "赋值就是引用" | 对，所有对象赋值都是引用拷贝 | 错，默认是 move；需要 `Rc` 才是共享引用 |
| "我可以随时修改对象" | 对，除非加了 final | 错，需要 `mut` 或 `RefCell`；借用检查器限制并发修改 |
| "对象在堆上长期存在" | 对，GC 决定何时释放 | 错，所有权决定何时释放；需要 `Rc/Arc` 才能延长生命周期 |
| "创建对象然后忘记" | 对，GC 会处理 | 错，必须考虑谁拥有它、何时释放 |

**具体的心智负担场景**：

1. **选择正确的智能指针组合**
   ```rust
   // 这个场景该用什么？
   // 单线程 + 共享 + 可变 = Rc<RefCell<T>>
   // 多线程 + 共享 + 可变 = Arc<Mutex<T>>
   // 独占 + 堆分配 = Box<T>
   ```
   Java 只有一种选择（`new`），Rust 有四种常见组合。

2. **生命周期标注**
   ```rust
   fn process(data: &Data) -> &Processed { ... } // 返回值能活多久？
   ```
   Java 从不考虑这个问题，GC 处理一切。Rust 要求你显式标注或推断引用的生命周期。

3. **借用检查器的限制**
   ```rust
   let mut data = vec![1, 2, 3];
   let first = &data[0];
   data.push(4); // 编译错误！因为 first 引用可能被 push 操作 invalid 掉
   println!("{}", first);
   ```
   这在 Java 中完全合法，因为 `ArrayList` 的扩容会在堆上重新分配，但旧引用仍然有效（GC 保证）。Rust 的 Vec 也可能重新分配，但编译器保守地禁止这种操作。

### 为什么这个负担是值得的？

**Java/OOP 模式的隐藏成本**：

```java
// 看起来简单的代码：
public void process(User user) {
    user.setName("Alice"); // 这个修改会影响到所有持有 user 引用的地方！
}
// 谁负责保证这个修改是安全的？
// - 如果另一个线程正在读取 user.name？
// - 如果 user 被用作 HashMap 的 key？
// - 如果 user 正在被序列化到数据库？
```

Java 的这些问题的答案都是：**运行时检测或开发者约定**。

**Rust 的答案**：编译器在编译期就阻止这些问题。

```rust
fn process(user: &mut User) {
    user.name = String::from("Alice");
}
// 编译器保证：
// - 当这个可变借用存在时，没有其他线程能访问 user
// - 当这个可变借用存在时，没有其他引用能读取 user
// - 如果 user 被 Rc<RefCell<...>> 包装，RefCell 在运行时检测并发借用
```

**心智负担的本质转移**：

| 阶段 | Java | Rust |
|------|------|------|
| 编码时 | 低心智负担（随意共享修改） | 高心智负担（考虑所有权） |
| 调试时 | 高心智负担（NPE、并发bug、内存泄漏） | 低心智负担（编译器已捕获大部分bug） |
| 运行时 | 高心智负担（GC停顿、异常处理） | 低心智负担（确定性行为） |

Rust 把负担从"调试和运行时"转移到了"编码时"。这是一次性支付 vs 持续支付的权衡。

---

## Rust 对 OOP 的超越：不是替代，是增强

### 类型状态模式：运行时检查 → 编译期检查

Java 的状态模式需要运行时检查 + 异常处理。Rust 的类型状态模式让非法状态**不可构造**。

### 所有权作为能力（Capability）

```rust
struct DatabaseToken; // 空类型，只有拥有者才能创建连接

struct Connection {
    _token: DatabaseToken, // 私有字段
}

// Connection 只能从受信任的模块构造
// 离开作用域时自动关闭——无法忘记
```

这种设计在 Java 中无法实现（至少无法在不依赖约定的情况下实现）。

### 零成本抽象

```rust
fn process<T: Drawable>(items: Vec<T>) { ... }
// 编译后生成具体类型的代码，性能 = 手写代码
```

Java 的泛型擦除意味着运行时类型丢失，接口调用有 vtable 开销。Rust 的静态分发默认零开销。

---

## 设计决策对比表

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| 默认所有权语义 | 独占（move） | 共享引用 | 共享引用（值传递）|
| 堆分配 | `Box<T>`（显式）| `new`（隐式）| `&T` / `make`（隐式）|
| 共享所有权 | `Rc<T>` / `Arc<T>`（显式）| 引用（默认）| 引用（默认）|
| 内部可变性 | `RefCell<T>` / `Mutex<T>`（显式）| `synchronized` / `volatile` | `sync.Mutex` |
| 工厂返回值 | `Box<dyn Trait>` | `InterfaceType` | `Interface` |
| 观察者列表 | `Vec<Rc<RefCell<dyn Observer>>>>` | `List<Observer>` | `[]Observer` |
| 状态模式 | 类型参数（编译期）| 运行时枚举检查 | 运行时检查 |
| 并发安全 | 编译期（Send/Sync）| 运行时（synchronized）| 运行时（channel/mutex）|
| 内存安全 | 编译期（所有权）| 运行时（GC + NPE）| 运行时（GC + panic）|
| 心智负担 | 编码时高，运行时低 | 编码时低，运行时高 | 编码时低，运行时中 |

---

## 常见误区

### 误区 1："Rust 不能用工厂模式"

**错误。** `Box<dyn Trait>` 就是工厂的返回值。差异在于调用者**拥有**返回的对象，而不是共享引用。如果需要共享语义，可以在 Box 外面再包一层 `Rc`。

### 误区 2："观察者模式需要 GC"

**错误。** `Rc` + `Weak` 可以实现安全的观察者模式。`Rc` 的引用计数会在最后一个引用消失时自动释放，不需要 GC。使用 `Weak` 可以避免循环引用导致的内存泄漏。

### 误区 3："Rust 不能表达循环数据结构"

**部分正确。** 纯所有权无法表达循环结构（如双向链表、图），但 `Rc` + `Weak` 或 `unsafe` 代码可以。标准库就有 `std::collections::LinkedList` 的实现。

### 误区 4："心智负担不值得"

**视场景而定。** 对于快速原型和脚本，Rust 的负担确实过重。但对于系统软件、网络服务、并发密集型应用，编译期的安全保证在长期使用中节省的心智负担远超编码时的投入。

---

## 运行

```bash
cargo run -p ownership_and_oop
```

---

## 总结

### 所有权不是 OOP 的敌人

Rust 的所有权系统**没有改变**设计模式的本质，它只是**显式化了**资源管理的语义：

- Java 默认"共享 + 可变 + GC"，开发者常常不知道自己依赖了这些语义
- Rust 要求你**显式选择**所有权模型：`Box`（独占）、`Rc`（共享单线程）、`Arc`（共享多线程）
- 一旦选择正确，编译器自动保证安全性——不需要在运行时担心

### 心智负担是真实的，但有限

学习 Rust 的心智负担集中在**前 3-6 个月**：
- 学习所有权、借用、生命周期的规则
- 学会选择正确的智能指针组合
- 适应编译器的"过度保守"（有时需要 `clone()` 或重构）

但一旦掌握，编码速度会回到正常水平，而且：
- 调试时间大幅减少（编译器已捕获 NPE、use-after-free、data race）
- 运行时行为更可预测（无 GC 停顿、无隐式异常）
- 并发代码可以放心编写（编译器保证线程安全）

### 最终建议

**不要试图在 Rust 中写 Java 代码。** 接受 Rust 的语义，用 Rust 的方式重新思考设计模式：

- 默认使用独占所有权（move），只在需要共享时引入 `Rc`/`Arc`
- 默认使用不可变借用（`&T`），只在需要修改时引入 `mut` 或 `RefCell`
- 利用类型状态模式将运行时检查转化为编译期保证
- 利用 Send/Sync 让编译器帮你验证并发安全

> **核心洞察**：Rust 不是"没有 OOP"，而是"OOP 的所有默认假设都被显式化了"。这种显式化增加了编码时的心智负担，但消除了调试和运行时的心智负担。对于长期维护的系统，这是一笔划算的买卖。
