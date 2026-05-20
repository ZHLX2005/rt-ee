// =============================================================================
// Rust 线程安全演示
//
// 设计意图：
// Rust 通过 Send/Sync trait 在编译时静态检查线程安全。
// 核心问题：如何防止数据竞争（data race）？
//
// 对比 Java：synchronized + volatile，运行时检查
// 对比 Go：channel + mutex，运行时检查
// Rust 的方案：编译时检查，零运行时开销
// =============================================================================

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

// =============================================================================
// 1. Send/Sync 基础
// =============================================================================

// 设计意图：Send 表示所有权可以在线程间安全转移
// 设计意图：Sync 表示可以在线程间安全共享引用
//
// 编译器自动推导：
// - 基本类型（i32, bool 等）：Send + Sync
// - 拥有所有权的类型（String, Vec 等）：Send + Sync
// - Arc<T>: Send + Sync（T 需要是 Send）
// - Mutex<T>: Send + Sync（T 需要是 Send）
// - Rc<T>: 不是 Send/Sync（引用计数不是原子的）
//
// 下面的函数展示哪些类型是 Send/Sync（通过实际行为验证）

fn demonstrate_send_sync() {
    println!("=== Send/Sync 基础 ===\n");

    // 证明 i32 是 Send：可以在线程间传递
    let _x: i32 = 42;
    println!("i32 可以安全在线程间传递（是 Send）");

    // 证明 String 是 Send
    let _s = String::from("hello");
    println!("String 可以安全在线程间传递（是 Send）");

    // Rc 不是 Send：下面的代码无法编译
    // use std::rc::Rc;
    // let rc = Rc::new(42);
    // thread::spawn(move || {
    //     println!("{}", rc); // 编译错误！Rc 不是 Send
    // });

    // Arc 是 Send：可以跨线程传递
    let _arc = Arc::new(42);
    println!("Arc<i32> 可以安全在线程间传递（是 Send）");

    println!();
}

// =============================================================================
// 2. 数据竞争与原子操作
// =============================================================================

// 设计意图：展示数据竞争的危险，以及如何用原子操作避免

fn data_race_demo() {
    println!("=== 数据竞争与原子操作 ===\n");

    // 错误方式：多个线程同时修改同一个非同步变量
    // 这会导致数据竞争，行为未定义
    //
    // let mut data = 0;  // 非原子
    // thread::spawn(|| { data += 1; });
    // thread::spawn(|| { data += 1; });

    // 正确方式：使用原子操作
    let counter = Arc::new(AtomicUsize::new(0));
    let mut handles = vec![];

    for _ in 0..4 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            for _ in 0..1000 {
                counter.fetch_add(1, Ordering::SeqCst); // 原子加法
            }
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("原子计数器结果: {} (应该是 4000)", counter.load(Ordering::SeqCst));
    println!("注释：原子操作避免了数据竞争\n");
}

// =============================================================================
// 3. Mutex：互斥锁
// =============================================================================

// 设计意图：Mutex 提供线程安全的共享状态
// - lock() 获取锁，返回 MutexGuard
// - MutexGuard 离开作用域时自动释放锁
// - 防止多个线程同时访问共享数据

fn mutex_demo() {
    println!("=== Mutex 演示 ===\n");

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..8 {
        let counter = Arc::clone(&counter);
        let handle = thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
            // MutexGuard 'num' 离开作用域，锁自动释放
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("Mutex 计数器结果: {} (应该是 8)", *counter.lock().unwrap());
    println!("注释：锁保证了同一时刻只有一个线程修改数据\n");
}

// =============================================================================
// 4. Arc：原子引用计数
// =============================================================================

// 设计意图：Arc 允许多个线程共享所有权
// - Rc 是单线程引用计数，不是 Send
// - Arc 是原子引用计数，可以跨线程使用
// - Arc::clone 增加引用计数，不会造成数据竞争

fn arc_demo() {
    println!("=== Arc 演示 ===\n");

    let data = Arc::new(vec![1, 2, 3, 4, 5]);
    let mut handles = vec![];

    for i in 0..3 {
        let data = Arc::clone(&data);
        let handle = thread::spawn(move || {
            println!("线程 {} 看到的数据: {:?}", i, data);
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("注释：Arc 允许多个线程同时读取共享数据\n");
}

// =============================================================================
// 5. Channel：消息传递
// =============================================================================

// 设计意图：channel 是一种"不要共享内存，传递消息"的设计
// - 发送端：tx.send(value)
// - 接收端：rx.recv()
// - 多个生产者，单个消费者（MPSC）

fn channel_demo() {
    println!("=== Channel 演示 ===\n");

    use std::sync::mpsc;

    let (tx, rx) = mpsc::channel();

    // 发送端 1
    let tx_clone = tx.clone();
    let handle1 = thread::spawn(move || {
        tx_clone.send(42).unwrap();
    });

    // 发送端 2 - 使用同一个 channel（类型必须一致）
    let handle2 = thread::spawn(move || {
        tx.send(100).unwrap();
    });

    handle1.join().unwrap();
    handle2.join().unwrap();

    println!("收到: {}", rx.recv().unwrap());
    println!("收到: {}", rx.recv().unwrap());
    println!("注释：多个发送者可以向同一个 channel 发送消息\n");
}

// =============================================================================
// 6. Send/Sync 的推导规则
// =============================================================================

// 设计意图：展示编译器如何自动推导 Send/Sync
//
// 推导规则：
// 1. 如果 T: Send，则 &T: Send
// 2. 如果 T: Sync，则 &T: Sync
// 3. 如果 T: Send + Sync，则 Arc<T>: Send + Sync
// 4. raw pointer 不是 Send/Sync（*const T, *mut T）

fn auto_trait_demo() {
    println!("=== Auto Trait 推导 ===\n");

    // 编译器自动推导 MyStruct 是 Send
    // 因为它的所有字段都是 Send
    struct MyStruct {
        x: i32,
        y: String,
    }

    println!("MyStruct 的所有字段都是 Send，所以 MyStruct 也是 Send");
    println!("&MyStruct 是 Sync，因为 MyStruct 是 Sync");

    // 如果有字段不是 Send，MyStruct 也不是 Send
    // struct NotSend {
    //     rc: Rc<i32>,  // Rc 不是 Send
    // }

    println!();
}

fn main() {
    println!("\n========================================");
    println!("Rust 线程安全机制演示");
    println!("========================================\n");

    demonstrate_send_sync();
    data_race_demo();
    mutex_demo();
    arc_demo();
    channel_demo();
    auto_trait_demo();

    println!("=== 设计启示 ===\n");
    println!("1. Send/Sync = 编译时线程安全标记");
    println!("2. Arc + Mutex = 线程安全的共享状态");
    println!("3. Channel = 消息传递，避免共享内存");
    println!("4. 原子类型 = 无锁并发");
    println!("\n对比 Java：运行时检查（synchronized）");
    println!("对比 Go：运行时检查（channel + mutex）");
    println!("Rust 的优势：编译时检查，零运行时开销");
}
