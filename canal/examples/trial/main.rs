use canal::trial::MyVec;

fn main() {
    let vec = MyVec::new();

    for i in 0..10000 {
        vec.push(i);
    }
}
