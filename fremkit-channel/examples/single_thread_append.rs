use fremkit_channel::unbounded::UnboundedChannel;
use log::info;

pub fn main() {
    env_logger::init();

    let channel: UnboundedChannel<u64> = UnboundedChannel::with_log_capacity(10);

    for i in 0..1_000 {
        info!("idx: {}", channel.push(i));
    }

    for i in 0..1_000 {
        info!("val: {:?}", channel.get(i));
    }
}
