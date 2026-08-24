fn main() {
    let v: Vec<Vec<i32>> = vec![Vec::with_capacity(32); 4];
    for x in &v {
        println!("cap: {}", x.capacity());
    }
}
