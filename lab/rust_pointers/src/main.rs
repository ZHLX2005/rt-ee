// =============================================================================
// Rust 指针类型全景演示
//
// Rust 中没有传统意义上的"指针"，而是多种具有不同语义和约束的引用类型。
// 核心原则：每种指针/引用都编码了一种特定的所有权语义。
// =============================================================================

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn main() {
    println!("=== Rust 指针类型全景 ===\n");

    demo_references();
    demo_box();
    demo_rc();
    demo_arc_mutex();
    demo_refcell();
    demo_cell();
    demo_raw_pointers();
    demo_function_pointers();

    println!("\n=== 指针选择速查表 ===");
    println!("独占所有权 + 堆分配        → Box<T>");
    println!("共享所有权 + 单线程        → Rc<T>");
    println!("共享所有权 + 多线程        → Arc<T>");
    println!("共享可变 + 单线程          → Rc<RefCell<T>>");
    println!("共享可变 + 多线程          → Arc<Mutex<T>>");
    println!("内部可变 + Copy 类型       → Cell<T>");
    println!("不安全场景                → *const T / *mut T");
    println!("回调/高阶函数             → fn(T) → U");
}

// =============================================================================
// 1. 引用（Reference）：借用而非拥有
// =============================================================================
// &T 和 &mut T 是 Rust 中最基础的"指针"，但它们不是指针——它们是借用（borrow）。
//
// 设计意图：
// - &T：不可变借用。允许多个读者同时存在，但不允许写者。
// - &mut T：可变借用。独占访问，同一时间只能有一个写者。
//
// 编译器在编译期检查借用规则，运行时零开销。
// 引用大小 = 1 个机器字（64 位 = 8 bytes）

fn demo_references() {
    println!("--- 1. 引用（&T / &mut T）：借用 ---");

    let s = String::from("hello");

    // 不可变引用：允许多个
    let r1 = &s;
    let r2 = &s;
    println!("两个不可变引用: {} {}", r1, r2);
    // r1, r2 在此处之后不再使用

    // 可变引用：独占
    let r3 = &mut s.clone();
    r3.push_str(" world");
    println!("可变引用: {}", r3);

    println!("大小: &String = {} bytes", std::mem::size_of::<&String>());
    println!("注释: 引用不是指针，而是有编译期约束的借用\n");
}

// =============================================================================
// 2. Box<T>：堆分配的独占所有权
// =============================================================================
// Box<T> 是在堆上分配内存并拥有所有权的智能指针。
//
// 设计意图：
// - 当数据太大不适合栈上分配时使用
// - 用于递归类型（如链表、树）
// - 实现动态分发（Box<dyn Trait>）
//
// 当 Box 离开作用域时，堆内存自动释放（Drop trait）。
// Box 大小 = 1 个机器字（指向堆的指针）

fn demo_box() {
    println!("--- 2. Box<T>：堆分配独占所有权 ---");

    let b = Box::new(42);
    println!("Box 中的值: {}", b);

    // Box 可以解引用
    let val = *b;
    println!("解引用后的值: {}", val);

    // 递归类型必须用 Box
    let list = Box::new(Node {
        value: 1,
        next: Some(Box::new(Node {
            value: 2,
            next: None,
        })),
    });
    println!("链表首节点: {}", list.value);

    println!("大小: Box<i32> = {} bytes", std::mem::size_of::<Box<i32>>());
    println!("注释: Box 就是 Rust 的 new，但有确定性析构\n");
}

struct Node {
    value: i32,
    next: Option<Box<Node>>,
}

// =============================================================================
// 3. Rc<T>：引用计数共享所有权（单线程）
// =============================================================================
// Rc（Reference Counted）允许多个所有者共享同一个值。
// 当最后一个引用被 drop 时，内存被释放。
//
// 设计意图：
// - 当数据的多个部分需要长期共享同一个值时使用
// - Rc 本身是不可变的；要可变需要配合 RefCell
// - 单线程专用（引用计数不是原子操作）
//
// Rc 大小 = 2 个机器字（指向堆的指针，但堆上还有引用计数）

fn demo_rc() {
    println!("--- 3. Rc<T>：单线程共享所有权 ---");

    let data = Rc::new(String::from("shared"));
    println!("初始引用计数: {}", Rc::strong_count(&data));

    {
        let data2 = Rc::clone(&data);
        println!("克隆后引用计数: {}", Rc::strong_count(&data));
        println!("data2 的值: {}", data2);
    } // data2 离开作用域，引用计数 -1

    println!("data2 drop 后计数: {}", Rc::strong_count(&data));
    println!("大小: Rc<String> = {} bytes", std::mem::size_of::<Rc<String>>());
    println!("注释: Rc.clone() 只增加引用计数，不拷贝数据\n");
}

// =============================================================================
// 4. Arc<T> + Mutex<T>：线程安全共享可变
// =============================================================================
// Arc（Atomic Reference Counted）：线程安全的 Rc
// Mutex：互斥锁，保证同一时间只有一个线程访问数据
//
// 组合效果：多线程之间安全地共享和修改数据。
// 编译器通过 Send/Sync trait 保证类型安全。

fn demo_arc_mutex() {
    println!("--- 4. Arc<Mutex<T>>：多线程共享可变 ---");

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..10 {
        let counter = Arc::clone(&counter);
        handles.push(std::thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        }));
    }

    for h in handles {
        h.join().unwrap();
    }

    println!("计数器: {}", *counter.lock().unwrap());
    println!("注释: Arc 用于跨线程共享，Mutex 用于互斥访问\n");
}

// =============================================================================
// 5. RefCell<T>：运行时借用检查（内部可变性）
// =============================================================================
// RefCell 提供了"内部可变性"：即使值本身是不可变的，
// 也可以通过 RefCell 在运行时获得可变访问。
//
// 设计意图：
// - 当编译器无法验证借用安全时使用（如回调函数、复杂的数据结构）
// - borrow() 返回 Ref<T>，borrow_mut() 返回 RefMut<T>
// - 运行时检查借用规则，违反时 panic
// - 单线程专用（不是线程安全的）

fn demo_refcell() {
    println!("--- 5. RefCell<T>：运行时借用检查 ---");

    let cell = RefCell::new(String::from("hello"));

    {
        let mut s = cell.borrow_mut();
        s.push_str(" world");
    }

    println!("修改后的值: {}", cell.borrow());

    // 下面这行如果 uncomment 会在运行时 panic：
    // let r1 = cell.borrow();
    // let mut r2 = cell.borrow_mut(); // panic! 已有不可变借用

    println!("注释: RefCell 在运行时检查借用，不是编译期\n");
}

// =============================================================================
// 6. Cell<T>：Copy 类型的内部可变性
// =============================================================================
// Cell 是比 RefCell 更轻量的内部可变性，但只适用于实现了 Copy trait 的类型。
//
// 原理：Cell 不是返回引用，而是整体拷贝值或替换值。
// 因此不存在同时存在多个引用的问题，不需要运行时检查。
// 开销极小。

fn demo_cell() {
    println!("--- 6. Cell<T>：Copy 类型的内部可变性 ---");

    let cell = Cell::new(42);
    println!("初始值: {}", cell.get());

    cell.set(100);
    println!("修改后: {}", cell.get());

    // Cell 不返回引用，而是拷贝值
    let val = cell.get(); // 拷贝出 100
    cell.set(val + 1);   // 替换为 101
    println!("递增后: {}", cell.get());

    println!("注释: Cell 只适用于 Copy 类型，开销最小\n");
}

// =============================================================================
// 7. 原始指针（Raw Pointer）：不安全的 C 风格指针
// =============================================================================
// *const T 和 *mut T 是 Rust 中最接近 C 指针的类型。
//
// 特点：
// - 不受借用检查器约束
// - 可能为 null
// - 可能悬空（指向已释放内存）
// - 解引用必须在 unsafe 块中
//
// 用途：
// - 与 C 代码交互（FFI）
// - 实现底层数据结构（如自定义内存分配器）
// - 极端性能优化场景

fn demo_raw_pointers() {
    println!("--- 7. 原始指针（*const T / *mut T）：不安全 ---");

    let x = 42;
    let r = &x as *const i32; // 从引用创建原始指针

    // 解引用原始指针必须在 unsafe 块中
    unsafe {
        println!("通过原始指针读取: {}", *r);
    }

    // 原始指针可以进行算术运算（像 C 指针一样）
    let arr = [1, 2, 3, 4, 5];
    let ptr = arr.as_ptr();
    unsafe {
        println!("ptr[0] = {}", *ptr);
        println!("ptr[2] = {}", *ptr.add(2));
    }

    println!("大小: *const i32 = {} bytes", std::mem::size_of::<*const i32>());
    println!("注释: 原始指针 = C 指针，所有安全检查由程序员负责\n");
}

// =============================================================================
// 8. 函数指针（Function Pointer）
// =============================================================================
// fn(T) → U 是函数指针类型，指向具体的函数地址。
//
// 与闭包（Fn/FnMut/FnOnce trait）的区别：
// - 函数指针：无捕获环境，可以强制转换
// - 闭包：可以捕获环境，是匿名类型

fn add_one(x: i32) -> i32 {
    x + 1
}

fn apply(f: fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn demo_function_pointers() {
    println!("--- 8. 函数指针（fn(T) → U）---");

    let f: fn(i32) -> i32 = add_one;
    println!("函数指针调用: {}", f(5));
    println!("通过参数传递: {}", apply(add_one, 10));

    // 函数指针大小和普通指针一样
    println!("大小: fn(i32) → i32 = {} bytes", std::mem::size_of::<fn(i32) -> i32>());
    println!("注释: 函数指针是具体的函数地址，与闭包不同\n");
}
