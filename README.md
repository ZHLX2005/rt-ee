# Rust Example

A collection of Rust examples for learning the Rust programming language.

## Getting Started

### Installation

Install Rust via [rustup](https://rustup.rs/):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### Running Examples

```bash
cargo run --example <example_name>
```

## Examples

### 1. Hello World
```rust
fn main() {
    println!("Hello, world!");
}
```

### 2. Variables and Mutability
```rust
fn main() {
    let x = 5;           // immutable
    let mut y = 10;      // mutable
    y = y + x;
    println!("x = {}, y = {}", x, y);
}
```

### 3. Data Types
```rust
fn main() {
    // Integer types
    let a: i32 = 42;
    let b: f64 = 3.14;

    // Boolean
    let is_rust_cool: bool = true;

    // Character
    let c: char = 'R';

    // Tuple
    let tuple: (i32, f64, char) = (42, 3.14, 'R');

    println!("a = {}, b = {}", a, b);
}
```

### 4. Functions
```rust
fn add(a: i32, b: i32) -> i32 {
    a + b
}

fn main() {
    let result = add(5, 3);
    println!("5 + 3 = {}", result);
}
```

### 5. Control Flow
```rust
fn main() {
    let number = 7;

    if number < 5 {
        println!("condition was true");
    } else {
        println!("condition was false");
    }

    // Loop
    let mut count = 0;
    loop {
        count += 1;
        if count == 3 {
            continue;
        }
        println!("count = {}", count);
        if count == 5 {
            break;
        }
    }
}
```

### 6. Ownership
```rust
fn main() {
    let s1 = String::from("hello");
    let s2 = s1;  // s1 is moved to s2

    // println!("{}", s1);  // Error: s1 is no longer valid
    println!("{}", s2);

    // Clone
    let s3 = s2.clone();
    println!("s2 = {}, s3 = {}", s2, s3);
}
```

### 7. Borrowing
```rust
fn main() {
    let s = String::from("hello");

    // Immutable borrow
    let len = calculate_length(&s);
    println!("Length of '{}' is {}", s, len);
}

fn calculate_length(s: &String) -> usize {
    s.len()
}
```

### 8. Struct
```rust
struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    fn area(&self) -> u32 {
        self.width * self.height
    }

    fn square(size: u32) -> Rectangle {
        Rectangle { width: size, height: size }
    }
}

fn main() {
    let rect = Rectangle { width: 30, height: 50 };
    println!("Area: {}", rect.area());

    let sq = Rectangle::square(10);
    println!("Square area: {}", sq.area());
}
```

### 9. Enums and Pattern Matching
```rust
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
    ChangeColor(i32, i32, i32),
}

fn main() {
    let msg = Message::Write(String::from("hello"));

    match msg {
        Message::Quit => println!("Quit"),
        Message::Move { x, y } => println!("Move to ({}, {})", x, y),
        Message::Write(text) => println!("Write: {}", text),
        Message::ChangeColor(r, g, b) => println!("Color: {}, {}, {}", r, g, b),
    }
}
```

### 10. Option and Result
```rust
fn main() {
    // Option
    let some_value: Option<i32> = Some(5);
    let no_value: Option<i32> = None;

    match some_value {
        Some(v) => println!("Got value: {}", v),
        None => println!("No value"),
    }

    // Result
    let result: Result<i32, &str> = Ok(42);
    match result {
        Ok(v) => println!("Success: {}", v),
        Err(e) => println!("Error: {}", e),
    }
}
```

### 11. Vector
```rust
fn main() {
    let mut v = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);

    println!("v = {:?}", v);
    println!("First: {:?}", v.first());
    println!("Last: {:?}", v.last());

    for i in &v {
        println!("Element: {}", i);
    }
}
```

### 12. HashMap
```rust
use std::collections::HashMap;

fn main() {
    let mut scores = HashMap::new();
    scores.insert(String::from("Blue"), 10);
    scores.insert(String::from("Yellow"), 50);

    println!("{:?}", scores);

    for (key, value) in &scores {
        println!("{}: {}", key, value);
    }
}
```

### 13. Error Handling
```rust
use std::fs::File;
use std::io::{self, Read};

fn read_file_contents(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

fn main() {
    match read_file_contents("Cargo.toml") {
        Ok(contents) => println!("{}", contents),
        Err(e) => println!("Error: {}", e),
    }
}
```

### 14. Generics
```rust
fn largest<T: PartialOrd>(list: &[T]) -> &T {
    let mut largest = &list[0];

    for item in list {
        if item > largest {
            largest = item;
        }
    }

    largest
}

fn main() {
    let numbers = vec![34, 50, 25, 100, 65];
    println!("Largest: {}", largest(&numbers));

    let chars = vec!['y', 'm', 'a', 'q'];
    println!("Largest: {}", largest(&chars));
}
```

### 15. Traits
```rust
trait Summary {
    fn summarize(&self) -> String;
}

struct Article {
    title: String,
    author: String,
}

impl Summary for Article {
    fn summarize(&self) -> String {
        format!("{} by {}", self.title, self.author)
    }
}

fn main() {
    let article = Article {
        title: String::from("Rust Programming"),
        author: String::from("Author"),
    };

    println!("{}", article.summarize());
}
```

### 16. Lifetimes
```rust
fn longest<'a>(x: &'a str, y: &'a str) -> &'a str {
    if x.len() > y.len() {
        x
    } else {
        y
    }
}

fn main() {
    let s1 = String::from("long string");
    let result;
    {
        let s2 = String::from("xyz");
        result = longest(s1.as_str(), s2.as_str());
        println!("Longest: {}", result);
    }
}
```

### 17. Iterators
```rust
fn main() {
    let v = vec![1, 2, 3, 4, 5];

    let sum: i32 = v.iter().sum();
    println!("Sum: {}", sum);

    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    println!("Doubled: {:?}", doubled);

    let filtered: Vec<i32> = v.iter().filter(|x| *x > 2).collect();
    println!("Filtered: {:?}", filtered);
}
```

### 18. Closures
```rust
fn main() {
    let x = 4;

    let equal_to_x = |z| z == x;

    println!("{}", equal_to_x(4));

    let closure = |a, b| {
        let result = a + b;
        result * 2
    };

    println!("{}", closure(2, 3));
}
```

### 19. Modules
```rust
mod outer {
    pub fn public_function() {
        println!("Public function");
    }

    fn private_function() {
        println!("Private function");
    }

    pub mod inner {
        pub fn inner_function() {
            println!("Inner function");
        }
    }
}

fn main() {
    outer::public_function();
    outer::inner::inner_function();
}
```

### 20. Testing
```rust
#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }

    #[test]
    #[should_panic]
    fn panic_test() {
        panic!("This test panics");
    }
}
```

## Resources

- [The Rust Programming Language Book](https://doc.rust-lang.org/book/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)
- [Rust Playground](https://play.rust-lang.org/)
