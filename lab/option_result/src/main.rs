fn main() {
    let some_value: Option<i32> = Some(5);
    match some_value {
        Some(v) => println!("Got value: {}", v),
        None => println!("No value"),
    }

    let result: Result<i32, &str> = Ok(42);
    match result {
        Ok(v) => println!("Success: {}", v),
        Err(e) => println!("Error: {}", e),
    }
}
