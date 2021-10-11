use canal::Canal;
use log::info;

pub fn main() {
    env_logger::init();

    let canal: Canal<u64> = Canal::with_capacity(10);

    for i in 0..1_000 {
        info!("idx: {}", canal.push(i));
    }

    for i in 0..1_000 {
        info!("val: {:?}", canal.get(i));
    }
}
