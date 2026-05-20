# 线程安全与 Send/Sync

## 设计背景与问题域

### 核心问题

多线程编程中最危险的问题是什么？**数据竞争（Data Race）**。

**数据竞争的定义**：
1. 两个或更多线程同时访问同一内存
2. 至少有一个访问是写操作
3. 没有任何同步机制

数据竞争会导致：
- 内存不安全（use-after-free, 双重释放）
- 不可预测的行为
- 难以复现的 bug

**Rust 的解决方案**：在编译时静态检查，阻止数据竞争的发生，而不是等到运行时才发现。

---

## 抽象设计分析

### Rust 的线程安全策略

| 策略 | 说明 |
|------|------|
| **Send/Sync Trait** | 编译器标记哪些类型可以安全地在线程间传递 |
| **借用检查器** | 确保没有数据竞争 |
| **Arc/Mutex** | 提供线程安全的共享状态 |
| **Channel** | 消息传递，避免共享内存 |

### Send Trait：线程间所有权的转移

```rust
// Send 表示类型可以安全地在线程间转移所有权
pub unsafe trait Send {}

// 例如：
// - i32 是 Send（Copy，栈上数据）
// - Rc 不是 Send（引用计数不是线程安全的）
// - Arc 是 Send（原子引用计数）
```

### Sync Trait：线程间的共享引用

```rust
// Sync 表示类型可以安全地在线程间共享引用
// &T 是 Send 时，T 就是 Sync
pub unsafe trait Sync {}

unsafe impl Send for MutexGuard<'_, T> {}
unsafe impl<T: ?Sized> Sync for Mutex<T> {}
```

---

## Send/Sync 的实现机制

### Auto Trait 机制

大多数类型**自动实现** Send 和 Sync：

```rust
// 编译器自动推导：
// - 如果 T: Send，则 &T: Send
// - 如果 T: Sync，则 &T: Sync
// - 如果 T: Send + Sync，则 Arc<T>: Send + Sync

struct MyStruct {
    x: i32,        // i32 是 Send
    y: String,     // String 是 Send（拥有所有权的类型）
}

// 自动推导：MyStruct 是 Send
// 因为所有字段都是 Send
```

### !Send 和 !Sync 的类型

```rust
//Rc<T> 不是线程安全的
use std::rc::Rc;

let shared = Rc::new(42);
// Rc::clone(&shared) 增加引用计数，但不是原子操作
// 如果在多线程中共享 Rc，会导致数据竞争

// 错误示例：
use std::thread;
let shared = Rc::new(42);
thread::spawn(|| {
    println!("{}", shared); // 编译错误！Rc 不是 Send
});
```

### 正确的替代：Arc

```rust
use std::sync::Arc;
use std::thread;

let shared = Arc::new(42);
let shared_clone = Arc::clone(&shared);

thread::spawn(move || {
    println!("{}", shared_clone); // OK！Arc 是 Send
});

println!("{}", shared); // 主线程仍然可以访问
```

### Arc vs Rc

| 特性 | Arc | Rc |
|------|-----|-----|
| 引用计数 | 原子操作（线程安全） | 非原子（仅单线程） |
| Send | ✅ | ❌ |
| Sync | ✅ | ❌ |
| 性能开销 | 更高 | 更低 |

---

## 数据竞争的实际例子

### 竞态条件

```rust
use std::thread;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

// 使用原子类型避免数据竞争
let counter = Arc::new(AtomicUsize::new(0));
let mut handles = vec![];

for _ in 0..10 {
    let counter = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        for _ in 0..1000 {
            counter.fetch_add(1, Ordering::SeqCst); // 原子操作
        }
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}

println!("{}", counter.load(Ordering::SeqCst)); // 10000
```

### 没有同步的竞态

```rust
// 这段代码有数据竞争，编译通过但运行时未定义
let mut data = 0;

let handle1 = thread::spawn(|| {
    for _ in 0..1000000 {
        data += 1; // 数据竞争！
    }
});

let handle2 = thread::spawn(|| {
    for _ in 0..1000000 {
        data += 1; // 数据竞争！
    }
});

handle1.join().unwrap();
handle2.join().unwrap();
println!("{}", data); // 结果不可预测！
```

**注意**：这段代码会编译通过（因为 `data` 是 i32，实现了 Send），但运行时行为是未定义的。

---

## 互斥锁：Mutex

### 基本用法

```rust
use std::sync::Mutex;
use std::thread;

let counter = Mutex::new(0);
let mut handles = vec![];

for _ in 0..8 {
    let counter = Arc::clone(&counter);
    let handle = thread::spawn(move || {
        let mut num = counter.lock().unwrap();
        *num += 1;
    });
    handles.push(handle);
}

for handle in handles {
    handle.join().unwrap();
}

println!("{}", *counter.lock().unwrap()); // 8
```

### MutexGuard 的生命周期

```rust
let mutex = Mutex::new(Vec::new());

// lock() 返回 MutexGuard
let guard = mutex.lock().unwrap();

// guard 离开作用域时自动释放锁
drop(guard);

// 或者使用 scoped thread
thread::scope(|s| {
    s.spawn(|| {
        let mut guard = mutex.lock().unwrap();
        guard.push(1);
    }); // guard 在这里自动 drop
});
```

---

## 通道：Channel

### 消息传递替代共享内存

```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel();

thread::spawn(move || {
    tx.send(42).unwrap();
});

println!("{}", rx.recv().unwrap()); // 42
```

### 多生产者单消费者

```rust
use std::sync::mpsc;
use std::thread;

let (tx, rx) = mpsc::channel();

for i in 0..3 {
    let tx = tx.clone();
    thread::spawn(move || {
        tx.send(i).unwrap();
    });
}

drop(tx); // 发送者全部 drop

for msg in rx {
    println!("{}", msg);
}
```

---

## 与 Java/Go 的对比

| 维度 | Rust | Java | Go |
|------|------|------|-----|
| **并发安全** | Send/Sync 编译时检查 | synchronized + volatile | channel + mutex |
| **数据竞争** | 编译错误 | 运行时检测（happens-before） | 运行时 panic |
| **空指针** | Option 类型 | NPE | nil |
| **内存模型** | 借用检查器 | JMM | Go memory model |
| **默认行为** | 不共享（Move） | 共享引用 | 共享内存 |

### Java 的 synchronized

```java
public class Counter {
    private int count = 0;

    public synchronized void increment() {
        count++; // synchronized 保证了原子性
    }
}
```

### Rust 的 Send + Mutex

```rust
use std::sync::Mutex;

struct Counter {
    count: Mutex<i32>,
}

impl Counter {
    fn increment(&self) {
        let mut num = self.count.lock().unwrap();
        *num += 1;
    }
}
```

### Go 的 channel

```go
func worker(ch chan int) {
    ch <- 42 // 发送
}

func main() {
    ch := make(chan int)
    go worker(ch)
    fmt.Println(<-ch) // 接收
}
```

---

## 设计哲学

### 为什么选择编译时检查？

1. **零运行时开销**：不需要额外的同步原语检查
2. **提前发现**：数据竞争在编译时就报错，不是运行时
3. **不可能的状态变成不可能的编译**：类似所有权系统

### 为什么不用 GC？

GC 只能追踪内存引用，不能追踪"哪些操作是原子的"。Rust 的方案更底层，更精确。

---

## 常见错误

### 错误 1：在线程间共享非 Send 类型

```rust
use std::thread;
use std::rc::Rc;

let shared = Rc::new(42);
thread::spawn(|| {
    println!("{}", shared); // 错误！Rc 不是 Send
});
```

### 错误 2：忘记加锁

```rust
use std::sync::Mutex;

let data = Mutex::new(vec![1, 2, 3]);

// 错误！直接访问内部数据
// data.push(4);

// 正确：
let mut guard = data.lock().unwrap();
guard.push(4);
```

### 错误 3：死锁

```rust
let a = Mutex::new(1);
let b = Mutex::new(2);

// 线程 1: lock a, then lock b
// 线程 2: lock b, then lock b
// 可能死锁！

// 解决方案：始终以相同顺序加锁
```

---

## 运行

```bash
cargo run -p thread_safety
```
