use canal::Canal;

fn main() {
    let vec = Canal::new();

    for i in 0..10000 {
        vec.push(i);
    }
}
