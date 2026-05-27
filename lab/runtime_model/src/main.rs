// Rust 运行时模型演示
//
// 核心问题：Rust 有类加载机制吗？
// 答案：没有。Rust 的所有类型在编译期就已完全确定，运行时没有类加载器。
//
// 这个程序演示 Rust 的"编译时确定一切"哲学，以及有限的运行时多态能力。

use std::any::{Any, TypeId};

// ============================================================
// 1. 编译时类型完全确定
// ============================================================
// Rust 的 struct + impl 不是 Java 的 class——没有继承体系，没有虚方法表（默认），
// 没有运行时类型信息。所有方法调用在编译时就解析为具体地址。

struct User {
    name: String,
    age: u32,
}

impl User {
    fn greet(&self) {
        println!("Hello, I'm {} and I'm {} years old", self.name, self.age);
    }
}

// 编译后，User::greet 的调用就是一个直接的函数调用，没有查找开销。
// 对比 Java：javac 编译为 invokevirtual 字节码，运行时由 JVM 解析为具体方法地址。

// ============================================================
// 2. 泛型的单态化（Monomorphization）
// ============================================================
// Rust 的泛型在编译期为每个具体类型生成独立代码。
// 不存在 Java "类型擦除"后的运行时类型丢失。

fn print_size<T>(_: &T) {
    println!("Size of T: {} bytes", std::mem::size_of::<T>());
}

// 编译后生成：
//   print_size::<i32>
//   print_size::<String>
//   print_size::<User>
// 每个都是独立的函数，内部直接使用具体类型的大小。
//
// 对比 Java：泛型擦除后运行时无法知道 List<String> 和 List<Integer> 的区别。
// 对比 Go：Go 1.18+ 也有泛型，但使用 GC-shape 而非完全单态化，减少代码膨胀。

// ============================================================
// 3. 有限的运行时多态：dyn Trait
// ============================================================
// Rust 默认不使用虚函数表（vtable），但可以通过 dyn Trait 显式请求运行时多态。
// 这是 Rust 中最接近 Java "类加载后动态绑定"的机制——但它仍然不是类加载。

trait Drawable {
    fn draw(&self);
}

struct Circle { radius: f64 }
struct Rectangle { width: f64, height: f64 }

impl Drawable for Circle {
    fn draw(&self) {
        println!("Drawing a circle with radius {}", self.radius);
    }
}

impl Drawable for Rectangle {
    fn draw(&self) {
        println!("Drawing a rectangle {}x{}", self.width, self.height);
    }
}

// 静态分发：编译时确定调用哪个 draw，零开销
fn render_static<T: Drawable>(item: &T) {
    item.draw(); // 直接调用 Circle::draw 或 Rectangle::draw
}

// 动态分发：运行时通过 vtable 查找 draw 的实现
// 这是 Rust 中最接近 Java interface 调用的机制
fn render_dynamic(items: &[Box<dyn Drawable>]) {
    for item in items {
        item.draw(); // vtable 查找：指针 → vtable → 函数地址
    }
}

// 关键区别：
// - Java：所有对象引用默认携带类型信息（对象头中的 Class pointer），方法调用默认虚调用
// - Rust：&dyn Trait 是一个"胖指针"（数据指针 + vtable 指针），必须显式构造
// - Rust 的 vtable 在编译时生成，运行时不会加载新的类型实现

// ============================================================
// 4. 有限的运行时类型信息：std::any::Any
// ============================================================
// Rust 的运行时类型信息极度有限，与 Java 的反射天差地别。
// TypeId 是编译时生成的唯一标识，运行时只能做等价比较，不能获取元数据。

fn type_name_of<T: Any>(value: &T) {
    let tid = TypeId::of::<T>();
    println!("TypeId of value: {:?}", tid);

    // 只能检查是否是已知类型，无法像 Java 反射那样遍历字段/方法
    // 需要将 &T 转换为 &dyn Any 才能使用 downcast_ref
    let any = value as &dyn Any;
    if let Some(s) = any.downcast_ref::<String>() {
        println!("It's a String: {}", s);
    } else if let Some(n) = any.downcast_ref::<i32>() {
        println!("It's an i32: {}", n);
    } else {
        println!("Unknown type to this function");
    }
}

// 对比 Java：
//   Class<?> clazz = obj.getClass();
//   for (Field f : clazz.getDeclaredFields()) { ... }  // 遍历所有字段
//   Method m = clazz.getMethod("foo");                // 获取方法并调用
//
// Rust 没有运行时类元数据，因为这些信息在编译后就不需要了。

// ============================================================
// 5. 为什么 Rust 不需要类加载？
// ============================================================
// Rust 程序编译后是一个完整的可执行文件（或静态/动态库）。
// 所有符号在链接时就已解析。运行时不需要：
// - 加载 .class 字节码文件
// - 解析符号引用
// - 验证字节码合法性
// - JIT 编译热点代码
//
// 这带来了根本性的差异：
// - Java 程序可以在运行时加载新的类（ClassLoader.loadClass）
// - Rust 程序运行时的类型集合在编译时就封闭了（除非使用 dlopen 动态加载 so/dll）

fn main() {
    println!("=== 1. 编译时类型确定 ===");
    let user = User {
        name: String::from("Alice"),
        age: 30,
    };
    user.greet(); // 编译时确定调用 User::greet

    println!("\n=== 2. 泛型单态化 ===");
    let n = 42i32;
    let s = String::from("hello");
    print_size(&n); // 编译为 print_size::<i32>
    print_size(&s); // 编译为 print_size::<String>
    print_size(&user); // 编译为 print_size::<User>

    println!("\n=== 3. 静态分发 vs 动态分发 ===");
    let circle = Circle { radius: 5.0 };
    let rect = Rectangle { width: 10.0, height: 20.0 };

    // 静态分发：编译器直接内联具体实现
    render_static(&circle);
    render_static(&rect);

    // 动态分发：运行时通过 vtable 查找
    let shapes: Vec<Box<dyn Drawable>> = vec![
        Box::new(Circle { radius: 3.0 }),
        Box::new(Rectangle { width: 4.0, height: 5.0 }),
    ];
    render_dynamic(&shapes);

    println!("\n=== 4. 有限的运行时类型信息 ===");
    type_name_of(&String::from("hello"));
    type_name_of(&42i32);
    type_name_of(&user); // 会走到 "Unknown type" 分支

    println!("\n=== 5. 内存布局对比 ===");
    println!("Size of User: {} bytes", std::mem::size_of::<User>());
    println!("Size of &User: {} bytes", std::mem::size_of::<&User>());
    println!("Size of &dyn Drawable: {} bytes", std::mem::size_of::<&dyn Drawable>());
    // &dyn Drawable 是胖指针：数据指针(8B) + vtable指针(8B) = 16B
    // Java 的对象引用：对象头(12-16B)包含类型信息 + 数据
}
