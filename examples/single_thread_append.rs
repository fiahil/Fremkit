use fremkit::bounded::Log;

const N: usize = 100;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let channel: Log<u64> = Log::new(N);

    for i in 0..N {
        println!("idx: {}", channel.push(i as u64)?);
    }

    for i in 0..N {
        println!("val: {:?}", channel.get(i));
    }

    Ok(())
}
