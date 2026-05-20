// Send/Sync 并发安全范式
//
// 设计意图：
// - 不依赖运行时检查或 GC，在编译期保证无数据竞争
// - Send/Sync 是标记 trait（Marker Trait），没有方法，仅表示类型属性
// - 编译器自动推导：如果类型的所有字段都是 Send，类型就是 Send

use std::sync::{Arc, Mutex};
use std::thread;

fn main() {
    // === 示例 1：Arc<Mutex<T>> 模式 ===
    // Arc：原子引用计数，允许多个线程共享所有权（Send + Sync）
    // Mutex：互斥锁，保证同一时间只有一个线程能访问数据
    // 组合起来：线程安全的共享可变状态

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for i in 0..5 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
            println!("Thread {} incremented counter to {}", i, *num);
            // MutexGuard 在这里 drop，锁自动释放
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Final counter: {}", *counter.lock().unwrap());

    // === 示例 2：Send 的约束 ===
    // Rc<T>（非原子引用计数）不是 Send，不能跨线程传递
    use std::rc::Rc;
    let rc = Rc::new(42);
    // 以下代码编译错误：
    // let rc2 = Rc::clone(&rc);
    // thread::spawn(move || { println!("{}", rc2); });
    // 错误信息：`*mut RcInner<i32>` cannot be sent between threads safely
    // 这是因为 Rc 的引用计数操作不是原子的，多线程操作会导致数据竞争

    // 解决：使用 Arc 替代 Rc
    let arc = Arc::new(42);
    let arc2 = Arc::clone(&arc);
    thread::spawn(move || {
        println!("Arc value in thread: {}", arc2);
    }).join().unwrap();

    // === 示例 3：Sync 的约束 ===
    // Cell<T> 提供内部可变性，但不是 Sync（不能安全地在线程间共享引用）
    use std::cell::Cell;
    let cell = Cell::new(42);
    // 以下代码编译错误：
    // let cell_ref = &cell;
    // thread::spawn(move || { cell_ref.set(100); });
    // 错误信息：`Cell<i32>` cannot be shared between threads safely

    // 解决：使用 Mutex<T> 或 Atomic 类型
    use std::sync::atomic::{AtomicI32, Ordering};
    let atomic = Arc::new(AtomicI32::new(42));
    let atomic2 = Arc::clone(&atomic);
    thread::spawn(move || {
        atomic2.store(100, Ordering::Relaxed);
    }).join().unwrap();

    println!("Atomic value: {}", atomic.load(Ordering::Relaxed));
}
