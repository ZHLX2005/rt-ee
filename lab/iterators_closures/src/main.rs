fn main() {
    let v = vec![1, 2, 3, 4, 5];
    let sum: i32 = v.iter().sum();
    println!("Sum: {}", sum);

    let doubled: Vec<i32> = v.iter().map(|x| x * 2).collect();
    println!("Doubled: {:?}", doubled);

    let x = 4;
    let equal_to_x = |z| z == x;
    println!("{}", equal_to_x(4));
}
