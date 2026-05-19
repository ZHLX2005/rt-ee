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
