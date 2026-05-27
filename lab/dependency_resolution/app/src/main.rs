// Rust 依赖版本冲突解决机制演示
//
// 核心问题：当外部库依赖低版本，自己的项目需要高版本，如何处理？
//
// Rust 的答案：不同 major 版本的同一个 crate 可以直接共存！
// 这是因为 Rust 的编译模型从底层就支持 crate 级别的版本隔离。

// 通过 Cargo.toml 中的 rename，我们同时引入了两个版本的 mylib：
// - mylib_v1 对应 mylib 1.0.0（Config + greet）
// - mylib_v2 对应 mylib 2.0.0（Settings + greet + set_verbose）
//
// 编译器将它们视为完全不同的类型系统实体，符号在链接阶段也被隔离。

use mylib_v1; // mylib v1.0.0
use mylib_v2; // mylib v2.0.0

fn main() {
    println!("=== Rust 多版本共存演示 ===\n");

    // v1 的 API
    let cfg = mylib_v1::Config::new("Alice");
    println!("v1: {}", cfg.greet());

    // v2 的 API（完全不同的类型，即使名字相同也不会冲突）
    let mut settings = mylib_v2::Settings::new("Bob");
    println!("v2: {}", settings.greet());
    settings.set_verbose(true);
    println!("v2 verbose: {}", settings.greet());

    println!("\n=== 关键洞察 ===");
    println!("mylib_v1::Config 和 mylib_v2::Settings 是完全不同的类型");
    println!("即使它们来自'同一个' crate 的不同版本，编译器也将其隔离");
    println!("这避免了 Java/Go 中常见的'依赖地狱'问题");

    // 类型隔离的证明：以下代码如果 uncomment 会编译失败
    // let _x: mylib_v1::Config = settings; // 错误：类型不匹配！

    // 这种隔离的代价：v1 和 v2 的类型不能隐式互通。
    // 如果需要桥接，必须手动写适配层（adapter pattern）。
}
