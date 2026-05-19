mod outer {
    pub fn public_function() {
        println!("Public function");
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
