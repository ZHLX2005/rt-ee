// mylib v2.0.0 —— 模拟同一个库的不兼容 major 版本
//
// v2 对 API 做了不兼容的重构：Config 改名为 Settings，greet 签名也变了。
// 在 Rust 中，v1 和 v2 可以同时在同一个二进制中存在，因为编译器将它们视为
// 完全不同的类型。

/// v2 的 Settings 结构体（v1 中叫 Config）
pub struct Settings {
    pub name: String,
    pub verbose: bool,
}

impl Settings {
    pub fn new(name: &str) -> Self {
        Settings {
            name: name.to_string(),
            verbose: false,
        }
    }

    /// v2 的问候方法，增加了 verbose 控制
    pub fn greet(&self) -> String {
        if self.verbose {
            format!("[VERBOSE] Greetings from v2, user: {}", self.name)
        } else {
            format!("Hi from v2, {}", self.name)
        }
    }

    /// v2 新增的方法
    pub fn set_verbose(&mut self, v: bool) {
        self.verbose = v;
    }
}
