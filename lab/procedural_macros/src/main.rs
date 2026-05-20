// Rust 宏机制演示：声明式宏 + 过程宏
//
// 设计意图：
// - 展示声明式宏（macro_rules!）的模式匹配能力
// - 展示过程宏（proc_macro）的 AST 变换能力
// - 演示 #[derive(Builder)] 从定义到使用的完整流程

use procedural_macros_derive::{Builder, trace_function};

// === 1. 声明式宏：macro_rules! ===

// 模拟 vec![] 的实现原理
macro_rules! my_vec {
    // 模式 1：空向量
    () => {
        Vec::new()
    };

    // 模式 2：vec![elem; n] — 重复元素
    ($elem:expr; $n:expr) => {
        {
            let n = $n;
            let mut v = Vec::with_capacity(n);
            for _ in 0..n {
                v.push($elem);
            }
            v
        }
    };

    // 模式 3：vec![a, b, c] — 列表
    ($($x:expr),+ $(,)?) => {
        {
            let mut v = Vec::new();
            $(v.push($x);)*
            v
        }
    };
}

// === 2. 过程宏：#[derive(Builder)] 的使用 ===

// 这里调用的不是预定义的 Builder，而是 procedural_macros_derive crate
// 在编译期生成的代码
#[derive(Builder)]
struct User {
    name: String,
    age: u32,
    email: String,
}

#[derive(Builder)]
struct ServerConfig {
    host: String,
    port: u16,
    workers: usize,
}

// === 3. 属性宏：#[trace_function] 的使用 ===

#[trace_function]
fn process_data(input: &str) -> String {
    input.to_uppercase()
}

#[trace_function]
fn compute_sum(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    println!("=== 声明式宏 ===");

    let v1: Vec<i32> = my_vec![];
    println!("Empty vec: {:?}", v1);

    let v2 = my_vec![42; 5];
    println!("Repeated vec: {:?}", v2);

    let v3 = my_vec![1, 2, 3, 4, 5];
    println!("List vec: {:?}", v3);

    println!("\n=== 过程宏：derive Builder ===");

    // UserBuilder 在编译期由 #[derive(Builder)] 生成
    let user = UserBuilder::new()
        .name("Alice".into())
        .age(30)
        .email("alice@example.com".into())
        .build();

    println!("Built user: name={}, age={}, email={}", user.name, user.age, user.email);

    // ServerConfigBuilder 同样由过程宏生成
    let config = ServerConfigBuilder::new()
        .host("127.0.0.1".into())
        .port(8080)
        .workers(4)
        .build();

    println!(
        "Built config: {}:{} with {} workers",
        config.host, config.port, config.workers
    );

    println!("\n=== 属性宏：trace_function ===");

    let result = process_data("hello");
    println!("Result: {}", result);

    let sum = compute_sum(10, 20);
    println!("Sum: {}", sum);

    println!("\n=== 关键洞察 ===");
    println!("声明式宏（macro_rules!）= 模式匹配 + 文本模板替换");
    println!("过程宏（proc_macro）= AST 解析 + AST 变换 + 代码生成");
    println!("两者都在编译期执行，不增加运行时开销");
}
