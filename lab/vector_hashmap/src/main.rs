use std::collections::HashMap;

fn main() {
    let mut v = vec![1, 2, 3, 4, 5];
    println!("v = {:?}", v);
    println!("First: {:?}", v.first());

    let mut scores = HashMap::new();
    scores.insert(String::from("Blue"), 10);
    scores.insert(String::from("Yellow"), 50);
    println!("{:?}", scores);
}
