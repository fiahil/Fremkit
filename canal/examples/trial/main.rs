use canal::trial::MySimpleBuffer;

fn main() {
    let vec = MySimpleBuffer::new();

    for _ in 0..10000 {
        vec.push(1);
    }
}
