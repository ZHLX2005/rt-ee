fn main() {
    let s1 = String::from("hello");
    let s2 = s1; // s1 is moved to s2

    println!("s2 = {}", s2);

    let s3 = s2.clone();
    println!("s2 = {}, s3 = {}", s2, s3);
}
