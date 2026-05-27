// =============================================================================
// Rust 所有权系统与 OOP 设计模式的兼容性演示
//
// 核心问题：
// 1. 所有权机制不会导致 OOP 设计模式失效吗？
// 2. 频繁转移和失效，心智负担不会很大吗？
//
// 答案：不会失效。Rust 通过智能指针系统提供了多种所有权语义，
// 可以在编译期保证安全的前提下，实现所有经典设计模式。
// 心智负担确实存在，但这是显式控制换取确定性的权衡。
// =============================================================================

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

fn main() {
    println!("=== Rust 所有权与 OOP 设计模式 ===\n");

    demo_box();
    demo_rc_refcell();
    demo_arc_mutex();
    demo_factory();
    demo_observer();
    demo_state_pattern();
    demo_strategy_pattern();
    demo_dependency_injection();

    println!("\n=== 总结 ===");
    println!("Rust 的所有权系统不是 OOP 的敌人，而是让资源管理显式化。");
    println!("通过智能指针组合，可以实现所有经典设计模式，");
    println!("同时获得编译期内存安全和并发安全的保证。");
}

// =============================================================================
// 1. Box<T>：堆分配与确定性析构
// =============================================================================
// Box<T> 是最简单的智能指针，它在堆上分配内存并拥有所有权。
// 当 Box 离开作用域时，堆内存被自动释放——这是所有权的基本应用。
//
// 对比 Java：new 创建的对象在堆上，GC 决定何时释放
// 对比 Go：&T 或 make() 在堆上，GC 决定何时释放
// Rust：Box 离开作用域时立即释放，时机确定

fn demo_box() {
    println!("--- 1. Box<T>：堆分配 ---");

    let boxed = Box::new(String::from("hello"));
    println!("Box 中的值: {}", boxed);
    // boxed 离开作用域时，String 的 drop 被调用，堆内存释放
    println!("注释: Box 提供了确定性析构\n");
}

// =============================================================================
// 2. Rc<RefCell<T>>：单线程共享可变所有权
// =============================================================================
// 这是 Rust 中最接近 Java "对象引用" 的模式。
//
// Rc<T>（Reference Counted）：多个所有者共享同一个值，引用计数为 0 时释放
// RefCell<T>：运行时借用检查，允许在不可变引用内部修改数据
//
// 组合效果：多个 Rc 指向同一个 RefCell，任何持有者都可以读写数据。
// 代价：引用计数有原子操作开销（Rc 是单线程的，开销较小）
//       RefCell 在运行时检查借用规则，违反时会 panic
//
// 对比 Java：Rc 相当于引用计数（不是 GC），RefCell 相当于没有 synchronized 的可变访问
// 注意：Rc<RefCell<T>> 只用于单线程！多线程用 Arc<Mutex<T>>

fn demo_rc_refcell() {
    println!("--- 2. Rc<RefCell<T>>：单线程共享可变 ---");

    // 创建一个共享的、可变的数据
    let data = Rc::new(RefCell::new(Vec::new()));

    // 克隆 Rc 指针（引用计数 +1），不是深拷贝数据
    let data2 = Rc::clone(&data);
    let data3 = Rc::clone(&data);

    // 通过任何一个 Rc 都可以修改数据
    data.borrow_mut().push(1);
    data2.borrow_mut().push(2);
    data3.borrow_mut().push(3);

    println!("共享数据: {:?}", data.borrow());
    println!("引用计数: {}", Rc::strong_count(&data));
    println!("注释: Rc 提供共享所有权，RefCell 提供内部可变性\n");
}

// =============================================================================
// 3. Arc<Mutex<T>>：多线程共享可变所有权
// =============================================================================
// Arc<T>（Atomic Reference Counted）：线程安全的 Rc
// Mutex<T>：互斥锁，保证同一时间只有一个线程能访问数据
//
// 这是 Rust 中多线程共享状态的标准模式。
// 编译器通过 Send/Sync trait 保证：只有线程安全的类型才能跨线程共享。

fn demo_arc_mutex() {
    println!("--- 3. Arc<Mutex<T>>：多线程共享可变 ---");

    let counter = Arc::new(Mutex::new(0));
    let mut handles = vec![];

    for _ in 0..5 {
        let counter = Arc::clone(&counter);
        let handle = std::thread::spawn(move || {
            let mut num = counter.lock().unwrap();
            *num += 1;
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    println!("计数器结果: {}", *counter.lock().unwrap());
    println!("注释: Arc + Mutex = 线程安全的共享可变状态\n");
}

// =============================================================================
// 4. 工厂模式
// =============================================================================
// 工厂模式在 Rust 中完全可行。
// 关键在于：工厂返回的可以是 Box<dyn Trait>，调用者获得所有权。
//
// 如果调用者需要共享，可以将 Box 放入 Rc/Arc。
// 这种"默认转移，按需共享"的设计比 Java 的"默认共享"更安全。

trait Animal {
    fn speak(&self);
}

struct Dog;
struct Cat;

impl Animal for Dog {
    fn speak(&self) { println!("Woof!"); }
}

impl Animal for Cat {
    fn speak(&self) { println!("Meow!"); }
}

enum AnimalType { Dog, Cat }

fn create_animal(kind: AnimalType) -> Box<dyn Animal> {
    match kind {
        AnimalType::Dog => Box::new(Dog),
        AnimalType::Cat => Box::new(Cat),
    }
}

fn demo_factory() {
    println!("--- 4. 工厂模式 ---");

    let animal = create_animal(AnimalType::Dog);
    animal.speak(); // 动态分发

    let animal = create_animal(AnimalType::Cat);
    animal.speak();

    println!("注释: Box<dyn Trait> 是工厂模式的标准返回类型\n");
}

// =============================================================================
// 5. 观察者模式
// =============================================================================
// 观察者模式需要多个对象持有对同一个 subject 的引用。
// 在 Rust 中，这通过 Rc<RefCell<Subject>> + Weak 指针实现。
//
// Weak 指针很重要：避免观察者之间循环引用导致内存泄漏。
// 当 Subject 被 drop 时，Weak 指针自动失效。

trait Observer {
    fn update(&self, message: &str);
}

struct EmailObserver;
impl Observer for EmailObserver {
    fn update(&self, message: &str) {
        println!("  [Email] 收到通知: {}", message);
    }
}

struct SmsObserver;
impl Observer for SmsObserver {
    fn update(&self, message: &str) {
        println!("  [SMS] 收到通知: {}", message);
    }
}

struct Subject {
    observers: Vec<Rc<RefCell<dyn Observer>>>,
}

impl Subject {
    fn new() -> Self {
        Subject { observers: vec![] }
    }

    fn attach(&mut self, observer: Rc<RefCell<dyn Observer>>) {
        self.observers.push(observer);
    }

    fn notify(&self, message: &str) {
        for observer in &self.observers {
            observer.borrow().update(message);
        }
    }
}

fn demo_observer() {
    println!("--- 5. 观察者模式 ---");

    let mut subject = Subject::new();

    let email: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(EmailObserver));
    let sms: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(SmsObserver));

    subject.attach(Rc::clone(&email));
    subject.attach(Rc::clone(&sms));

    subject.notify("新订单到达!");
    println!("注释: Rc<RefCell<dyn Observer>> 实现观察者列表\n");
}

// =============================================================================
// 6. 状态模式（类型状态）
// =============================================================================
// Rust 的类型系统可以将状态编码到类型中。
// 这比运行时检查更安全——非法状态转换在编译期就被阻止。
//
// 对比 Java：用 enum + 运行时检查
// Rust：用泛型参数 + 编译期检查

struct Draft;
struct PendingReview;
struct Published;

struct Post<State> {
    content: String,
    _state: std::marker::PhantomData<State>,
}

impl Post<Draft> {
    fn new() -> Self {
        Post { content: String::new(), _state: std::marker::PhantomData }
    }

    fn add_text(&mut self, text: &str) {
        self.content.push_str(text);
    }

    fn request_review(self) -> Post<PendingReview> {
        Post { content: self.content, _state: std::marker::PhantomData }
    }
}

impl Post<PendingReview> {
    fn approve(self) -> Post<Published> {
        Post { content: self.content, _state: std::marker::PhantomData }
    }
}

impl Post<Published> {
    fn content(&self) -> &str {
        &self.content
    }
}

fn demo_state_pattern() {
    println!("--- 6. 状态模式（类型状态）---");

    let mut post = Post::new();
    post.add_text("I ate a salad for lunch today");

    let post = post.request_review();
    let post = post.approve();

    println!("发布内容: {}", post.content());

    // 下面这行如果 uncomment 会编译失败：
    // post.add_text("追加内容"); // 错误：Post<Published> 没有 add_text 方法！

    println!("注释: 状态转换在类型系统中编码，非法操作编译失败\n");
}

// =============================================================================
// 7. 策略模式
// =============================================================================
// 策略模式在 Rust 中可以通过 trait + Box<dyn Trait> 实现。
// 也可以用泛型实现静态分发（零成本）。

trait PaymentStrategy {
    fn pay(&self, amount: f64);
}

struct CreditCard { number: String }
struct PayPal { email: String }

impl PaymentStrategy for CreditCard {
    fn pay(&self, amount: f64) {
        println!("使用信用卡 {} 支付 {:.2}", self.number, amount);
    }
}

impl PaymentStrategy for PayPal {
    fn pay(&self, amount: f64) {
        println!("使用 PayPal {} 支付 {:.2}", self.email, amount);
    }
}

struct ShoppingCart {
    strategy: Box<dyn PaymentStrategy>,
}

impl ShoppingCart {
    fn with_strategy(strategy: Box<dyn PaymentStrategy>) -> Self {
        ShoppingCart { strategy }
    }

    fn checkout(&self, amount: f64) {
        self.strategy.pay(amount);
    }
}

fn demo_strategy_pattern() {
    println!("--- 7. 策略模式 ---");

    let cart = ShoppingCart::with_strategy(
        Box::new(CreditCard { number: "1234-5678".to_string() })
    );
    cart.checkout(100.0);

    let cart = ShoppingCart::with_strategy(
        Box::new(PayPal { email: "user@example.com".to_string() })
    );
    cart.checkout(50.0);

    println!("注释: Box<dyn Trait> 实现运行时策略切换\n");
}

// =============================================================================
// 8. 依赖注入
// =============================================================================
// 依赖注入在 Rust 中非常自然——通过 trait 和构造函数注入。
// 由于所有权的存在，注入的依赖生命周期被编译器精确追踪。

trait Repository {
    fn find_by_id(&self, id: u64) -> Option<String>;
}

struct InMemoryRepository {
    data: RefCell<std::collections::HashMap<u64, String>>,
}

impl InMemoryRepository {
    fn new() -> Self {
        InMemoryRepository { data: RefCell::new(std::collections::HashMap::new()) }
    }
}

impl Repository for InMemoryRepository {
    fn find_by_id(&self, id: u64) -> Option<String> {
        self.data.borrow().get(&id).cloned()
    }
}

struct UserService<R: Repository> {
    repo: R,
}

impl<R: Repository> UserService<R> {
    fn new(repo: R) -> Self {
        UserService { repo }
    }

    fn get_user(&self, id: u64) -> Option<String> {
        self.repo.find_by_id(id)
    }
}

fn demo_dependency_injection() {
    println!("--- 8. 依赖注入 ---");

    let repo = InMemoryRepository::new();
    let service = UserService::new(repo);

    // 可以注入不同的 Repository 实现，无需修改 UserService
    println!("查找用户: {:?}", service.get_user(1));

    println!("注释: 泛型参数实现编译期依赖注入，零运行时开销\n");
}
