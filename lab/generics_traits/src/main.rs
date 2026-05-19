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
