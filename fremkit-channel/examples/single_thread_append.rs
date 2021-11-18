use fremkit_channel::bounded::Channel;

const N: usize = 10_000;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel: Channel<u64> = Channel::new(N);

    for i in 0..N {
        println!("idx: {}", channel.push(i as u64)?);
    }

    for i in 0..N {
        println!("val: {:?}", channel.get(i));
    }

    Ok(())
}
