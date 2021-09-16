
use canal::Canal;

#[test]
fn test_canal() {
    let canal = Canal::new();
    let cd = Cooldown::new(2);
    let (c1, c2) = (canal.clone(), canal.clone());
    let (cd1, cd2) = (cd.clone(), cd.clone());

    let h1 = thread::spawn(move || {
        // starts threads simultaneously
        cd1.ready();

        let mut i = 0;

        while i < 10 {
            c1.add(1).unwrap();
            i += 1;
        }

        i
    });

    let h2 = thread::spawn(move || {
        // starts threads simultaneously
        cd2.ready();

        let mut i = 0;

        loop {
            let x = c2.get_blocking(i);
            println!("## {:?}", x);

            i += 1;

            if i == 10 {
                break;
            }
        }

        i
    });

    cd.wait();
    assert_eq!(h1.join().unwrap(), 10);
    assert_eq!(h2.join().unwrap(), 10);
}
