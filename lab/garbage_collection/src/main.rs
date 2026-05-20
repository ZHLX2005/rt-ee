// Rust 与垃圾回收（GC）
//
// 设计意图：
// - 展示 Rust 没有 GC，但有自动内存管理
// - 展示 Rc/Arc 引用计数及其局限性
// - 展示循环引用导致的内存泄漏
// - 展示 Weak 如何打破循环引用
// - 对比确定性析构 vs GC 的非确定性

use std::cell::RefCell;
use std::rc::{Rc, Weak};

// === 1. 所有权 + 确定性析构 ===

fn demo_deterministic_destruction() {
    println!("=== 1. 确定性析构 ===");

    {
        let data = vec![1, 2, 3, 4, 5];
        println!("Using data: {:?}", data);
    } // data 在这里立即释放，编译器自动插入 drop(data)

    println!("data 已释放（没有 GC，没有暂停）");
}

// === 2. Rc：引用计数 ===

fn demo_reference_counting() {
    println!("\n=== 2. Rc 引用计数 ===");

    let data = Rc::new(vec![1, 2, 3]);
    println!("创建后引用计数: {}", Rc::strong_count(&data));

    {
        let data2 = Rc::clone(&data);
        println!("clone 后引用计数: {}", Rc::strong_count(&data));
        println!("data2: {:?}", data2);
    } // data2 离开作用域，引用计数 -1

    println!("data2 离开后引用计数: {}", Rc::strong_count(&data));
    println!("data: {:?}", data);
} // data 离开作用域，引用计数归零，内存释放

// === 3. 循环引用导致的内存泄漏 ===

struct Node {
    value: i32,
    next: Option<Rc<RefCell<Node>>>,
}

fn demo_circular_reference_leak() {
    println!("\n=== 3. 循环引用导致内存泄漏 ===");

    let a = Rc::new(RefCell::new(Node { value: 1, next: None }));
    let b = Rc::new(RefCell::new(Node { value: 2, next: None }));

    println!("a 引用计数: {}", Rc::strong_count(&a));
    println!("b 引用计数: {}", Rc::strong_count(&b));

    // 创建循环引用
    a.borrow_mut().next = Some(Rc::clone(&b));
    b.borrow_mut().next = Some(Rc::clone(&a));

    println!("循环引用后 a 引用计数: {}", Rc::strong_count(&a));
    println!("循环引用后 b 引用计数: {}", Rc::strong_count(&b));

    // a 和 b 的 Rc 变量离开作用域
    // 但 a 和 b 的引用计数不会归零（互相持有引用）
    // 内存泄漏！
    println!("a 和 b 离开作用域，但内存不会被释放（泄漏）");
}

// === 4. Weak：打破循环引用 ===

struct NodeWithWeak {
    value: i32,
    // parent 使用 Weak，不增加引用计数
    parent: Option<Weak<RefCell<NodeWithWeak>>>,
    children: Vec<Rc<RefCell<NodeWithWeak>>>,
}

fn demo_weak_reference() {
    println!("\n=== 4. Weak 打破循环引用 ===");

    let root = Rc::new(RefCell::new(NodeWithWeak {
        value: 1,
        parent: None,
        children: vec![],
    }));

    {
        let child = Rc::new(RefCell::new(NodeWithWeak {
            value: 2,
            parent: Some(Rc::downgrade(&root)), // Weak 引用
            children: vec![],
        }));

        root.borrow_mut().children.push(Rc::clone(&child));

        println!("root strong_count: {}", Rc::strong_count(&root));
        println!("child strong_count: {}", Rc::strong_count(&child));
        // root 的引用计数不会因为 child.parent 而增加
        // 因为 parent 是 Weak

        // 从 child 访问 parent
        if let Some(parent_weak) = &child.borrow().parent {
            if let Some(parent) = parent_weak.upgrade() {
                println!("child's parent value: {}", parent.borrow().value);
            }
        }

        println!("child 离开作用域...");
    } // child 的 Rc 离开作用域

    // root 的 children vec 仍然持有 child 的 Rc
    // 但如果我们清空 children，child 就会被释放
    println!("root 仍然存在，value: {}", root.borrow().value);
    println!("root strong_count: {}", Rc::strong_count(&root));
}

// === 5. Drop trait：通用资源管理 ===

struct DatabaseConnection {
    id: u32,
}

impl Drop for DatabaseConnection {
    fn drop(&mut self) {
        println!("   数据库连接 #{} 已关闭", self.id);
    }
}

fn demo_drop_trait() {
    println!("\n=== 5. Drop trait：通用资源管理 ===");

    {
        let conn1 = DatabaseConnection { id: 1 };
        let conn2 = DatabaseConnection { id: 2 };
        println!("使用连接 #{} 和 #{}", conn1.id, conn2.id);
    } // conn2 先 drop，然后 conn1（LIFO 顺序）

    println!("所有连接已自动关闭（没有 GC 延迟）");
}

// === 6. 内存使用对比 ===

fn demo_memory_comparison() {
    println!("\n=== 6. 内存使用对比 ===");

    // Vec 的内存布局：栈上有指针+长度+容量，堆上有实际数据
    let vec = vec![0u8; 1024];
    println!("Vec 本身大小（栈）: {} bytes", std::mem::size_of_val(&vec));
    println!("Vec 数据大小（堆）: {} bytes", vec.capacity());

    // Rc 的内存布局：堆上有引用计数+数据
    let rc = Rc::new(vec![0u8; 1024]);
    println!("Rc 本身大小（栈）: {} bytes", std::mem::size_of_val(&rc));
    println!("Rc 引用计数+数据（堆）: {} bytes + 开销", rc.capacity());

    // Box 的内存布局：堆上有数据，栈上只有指针
    let boxed = Box::new([0u8; 1024]);
    println!("Box 本身大小（栈）: {} bytes", std::mem::size_of_val(&boxed));
    println!("Box 数据大小（堆）: {} bytes", boxed.len());
}

fn main() {
    demo_deterministic_destruction();
    demo_reference_counting();
    demo_circular_reference_leak();
    demo_weak_reference();
    demo_drop_trait();
    demo_memory_comparison();

    println!("\n=== 关键洞察 ===");
    println!("Rust 没有 GC，但有三种自动内存管理方式：");
    println!("  1. 所有权 + Drop —— 编译期确定性释放（零运行时开销）");
    println!("  2. Rc/Arc —— 引用计数（立即释放，但无法处理循环引用）");
    println!("  3. Weak —— 弱引用（打破循环引用）");
    println!("");
    println!("与 GC 的根本区别：");
    println!("  - GC：运行时扫描堆内存，推测式回收");
    println!("  - Rust：编译期确定释放时机，确定性回收");
    println!("  - GC 有暂停，Rust 无暂停");
    println!("  - GC 自动处理循环引用，Rust 需要 Weak 手动处理");
}
