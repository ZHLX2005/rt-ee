// =============================================================================
// Rust Trait 机制演示
//
// 设计意图：
// Trait 是 Rust 的行为抽象机制，类似于 Java 的接口但更强大。
// 核心问题：如何定义"可以被某种方式使用的类型"？
//
// 对比 Java：接口可以定义行为，但不支持默认实现（Java 8 之前）和关联类型
// 对比 Go：隐式接口更灵活，但缺乏编译时检查，容易意外实现
// Rust 的方案：名义性 trait + 默认实现 + 关联类型 + 零成本抽象
// =============================================================================

use std::fmt::{Debug, Display};

// =============================================================================
// 1. Trait 基础：定义行为契约
// =============================================================================

// 定义一个 trait：可以被总结的类型
trait Summary {
    // 核心方法：返回摘要
    fn summarize(&self) -> String;

    // 默认实现：提供可覆盖的默认行为
    // 设计意图：减少样板代码，类似 Java 8 的 default 方法
    fn summarize_author(&self) -> String {
        String::from("(Unknown Author)")
    }
}

struct Article {
    title: String,
    author: String,
    content: String,
}

// 为 Article 实现 Summary trait
// 设计意图：显式声明 Article 承诺提供摘要行为
impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{}, by {}", self.title, self.author)
    }

    // 覆盖默认实现
    fn summarize_author(&self) -> String {
        format!("Written by {}", self.author)
    }
}

struct Tweet {
    username: String,
    content: String,
}

// 设计意图：Tweet 和 Article 实现了相同的 trait
// 但具体行为不同——trait 只定义契约，具体实现由类型决定
impl Summary for Tweet {
    fn summarize(&self) -> String {
        format!("@{}: {}", self.username, self.content)
    }

    // 使用默认的 summarize_author 实现
}

fn print_summary(item: &impl Summary) {
    // 设计意图：impl Trait 语法糖
    // 编译器检查 item 是否实现了 Summary
    // 这是静态分发的语法形式
    println!("Summary: {}", item.summarize());
    println!("Author: {}", item.summarize_author());
}

// =============================================================================
// 2. 静态分发 vs 动态分发
// =============================================================================

// 静态分发：泛型在编译时展开
// 设计意图：每个具体类型生成专用代码，无虚函数调用开销
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];
    for item in list {
        // T 必须实现 PartialOrd 才能比较
        if item > largest {
            largest = item;
        }
    }
    largest
}

// 动态分发：运行时查找方法
// 设计意图：统一处理多种类型，但有 vtable 查找开销
fn notify(item: &dyn Summary) {
    // dyn Summary 创建 trait object
    // 运行时通过 vtable 查找 summarize()
    println!("Notify: {}", item.summarize());
}

// =============================================================================
// 3. 关联类型：定义迭代器等场景的核心机制
// =============================================================================

// 设计意图：为什么需要关联类型？
// 考虑：如果用泛型参数
//   trait Iterator<T> { fn next(&mut self) -> Option<T>; }
// 调用者必须指定：let iter: Iterator<String> = ...;
// 这意味着一个类型只能有一种迭代行为

// 使用关联类型：
//   trait Iterator { type Item; }
// 调用者通过 <Type as Iterator>::Item 获取类型
// 一个类型可以有不同的迭代方式（通过不同的 trait）

trait Iterator {
    type Item; // 关联类型：实现者定义具体类型

    fn next(&mut self) -> Option<Self::Item>;
}

struct Counter {
    count: usize,
    max: usize,
}

impl Iterator for Counter {
    type Item = usize; // 明确指定 Item 类型

    fn next(&mut self) -> Option<Self::Item> {
        if self.count < self.max {
            self.count += 1;
            Some(self.count)
        } else {
            None
        }
    }
}

// 设计意图：关联类型确保了"一致性"
// Counter 只能有一种 Item 类型，无法同时是 usize 和 String

// =============================================================================
// 4. Trait 约束与 where 子句
// =============================================================================

// 多个约束：T 必须同时实现 Display 和 Debug
fn print_debug<T: Display + Debug>(value: T) {
    println!("Display: {}", value);
    println!("Debug: {:?}", value);
}

// where 子句：更清晰的约束表达
fn print_all<T>(items: &[T])
where
    T: Display + Debug,
{
    for item in items {
        println!("{}", item);
    }
}

// =============================================================================
// 5. Trait 对象：异质集合
// =============================================================================

// 设计意图：如何存储不同类型但实现同一 trait 的值？
// Vec<Article> 只能存 Article
// Vec<&dyn Summary> 可以存任何实现 Summary 的类型的引用

fn main() {
    println!("=== Trait 基础 ===\n");

    let article = Article {
        title: String::from("Rust Programming"),
        author: String::from("Alice"),
        content: String::from("..."),
    };

    let tweet = Tweet {
        username: String::from("bob"),
        content: String::from("Hello world!"),
    };

    println!("Article:");
    print_summary(&article);

    println!("\nTweet:");
    print_summary(&tweet);

    println!("\n=== 静态分发 vs 动态分发 ===\n");

    let numbers = vec![1, 5, 3, 2, 4];
    println!("largest number: {}", largest(&numbers));

    // 动态分发：notify 接受任何实现 Summary 的类型
    notify(&article);
    notify(&tweet);

    println!("\n=== 关联类型 ===\n");

    let mut counter = Counter { count: 0, max: 3 };
    println!("Counter iteration:");
    while let Some(n) = counter.next() {
        println!("  {}", n);
    }

    println!("\n=== Trait 约束 ===\n");

    print_debug(42i32);
    print_debug("hello");

    println!("\n=== 设计启示 ===\n");
    println!("1. Trait = 行为抽象的契约（类似 Java Interface）");
    println!("2. 默认实现 = 减少样板代码（Java 8+ 的 default）");
    println!("3. 关联类型 = 绑定到类型的类型（Iterator::Item）");
    println!("4. 静态分发 = 泛型展开，零开销（monomorphization）");
    println!("5. 动态分发 = trait object，灵活但有 vtable 开销");
    println!("\n对比 Java：trait 更强大（默认实现、关联类型、零成本抽象）");
    println!("对比 Go：显式 impl 更安全，编译时检查更严格");
}

// =============================================================================
// 6. 高级示例：Orphan Rule 演示
// =============================================================================

// 设计意图：Rust 的 Orphan Rule 防止意外实现冲突
//
// 假设我们想为 Vec<String> 实现 Display
// 以下代码会编译失败，因为 Vec 和 Display 都来自标准库
//
// impl Display for Vec<String> {
//     fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
//         write!(f, "[{}]", self.join(", "))
//     }
// }
//
// 这样做是为了避免"第三方库意外实现冲突"的问题
//
// 正确做法：为自己的类型实现别人的 trait，或为别人的类型实现自己的 trait
//

// 为我们自己的类型实现标准库的 trait
struct Wrapper(Vec<String>);

impl Display for Wrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[{}]", self.0.join(", "))
    }
}
