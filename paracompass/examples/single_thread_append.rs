use log::info;
use paracompass::Channel;

pub fn main() {
    env_logger::init();

    let channel: Channel<u64> = Channel::with_capacity(10);

    for i in 0..1_000 {
        info!("idx: {}", channel.push(i));
    }

    for i in 0..1_000 {
        info!("val: {:?}", channel.get(i));
    }
}
