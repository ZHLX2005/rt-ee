// mylib v1.0.0 —— 模拟一个库的初代 API
//
// 设计意图：展示不同 major 版本的同一个 crate 可以共存于一个项目中。
// Cargo 通过版本号将它们隔离为完全不同的编译单元。

/// v1 的 Config 结构体
pub struct Config {
    pub name: String,
}

impl Config {
    pub fn new(name: &str) -> Self {
        Config {
            name: name.to_string(),
        }
    }

    /// v1 的问候方法
    pub fn greet(&self) -> String {
        format!("Hello from v1, {}!", self.name)
    }
}
