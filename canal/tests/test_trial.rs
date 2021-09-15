use log::debug;

use canal::trial::{MyBuffer, MySimpleBuffer, MySuperBuffer, MyVec};

use std::sync::Arc;

use loom;
use loom::thread;

fn init() {
    let _ = env_logger::builder().is_test(true).try_init();
}

#[test]
fn test_vec() {
    init();

    loom::model(|| {
        let vec = MyVec::new();

        vec.push(1);
        vec.push(2);
        vec.push(3);

        assert_eq!(vec.get(0), Some(&1));
        assert_eq!(vec.get(1), Some(&2));
        assert_eq!(vec.get(2), Some(&3));
        assert_eq!(vec.get(3), None);
    });
}

#[test]
/// run test for eventually consistent commit log
fn test_eventual_consistency_log() {
    init();

    loom::model(|| {
        let vec = Arc::new(MyVec::new());
        let v1 = vec.clone();
        let v2 = vec.clone();

        let h1 = thread::spawn(move || {
            v1.push('a');

            let x0 = v1.get(0);
            let x1 = v1.get(1);

            (x0.cloned(), x1.cloned())
        });

        let h2 = thread::spawn(move || {
            v2.push('b');

            let x0 = v2.get(0);
            let x1 = v2.get(1);

            (x0.cloned(), x1.cloned())
        });

        let (x0h1, x1h1) = h1.join().unwrap();
        let (x0h2, x1h2) = h2.join().unwrap();
        let (x0, x1) = (vec.get(0), vec.get(1));

        debug!(
            "0: h1(a) {:<10} h2(c) {:<10}  f {:<10}",
            format!("{:?}", x0h1),
            format!("{:?}", x0h2),
            format!("{:?}", x0)
        );
        debug!(
            "1: h1(b) {:<10} h2(d) {:<10}  f {:<10}",
            format!("{:?}", x1h1),
            format!("{:?}", x1h2),
            format!("{:?}", x1)
        );
        debug!("");

        match (x0h1, x1h1, x0h2, x1h2) {
            (None, None, _, _) | (_, _, None, None) => {
                assert!(false, "1|2: (Read your own write)");
            }
            (None, Some(_), None, Some(_)) => {
                assert!(false, "1: (Read your own write)");
            }
            (Some(_), None, Some(_), None) => {
                assert!(false, "2: (Read your own write)");
            }
            (None, Some(_), Some(_), None) => {
                // TODO: WTH? problem with loom?
                assert!(false, "(Observed state are global)");
            }

            (Some(a), None, None, Some(d)) => {
                assert_eq!(Some(&a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(&d), x1, "d == x1 (Observed state are immutable)");
            }
            (None, Some(b), Some(c), Some(d)) => {
                assert_eq!(b, d, "b == d (Observed state are in-order)");
                assert_eq!(Some(&b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(&c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(&d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), None, Some(c), Some(d)) => {
                assert_eq!(a, c, "a == c (Observed state are in-order)");
                assert_eq!(Some(&a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(&c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(&d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), Some(b), Some(c), None) => {
                assert_eq!(a, c, "a == c (Observed state are in-order)");
                assert_eq!(Some(&a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(&b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(&c), x0, "c == x0 (Observed state are immutable)");
            }
            (Some(a), Some(b), None, Some(d)) => {
                assert_eq!(b, d, "b == d (Observed state are in-order)");
                assert_eq!(Some(&a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(&b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(&d), x1, "d == x1 (Observed state are immutable)");
            }
            (Some(a), Some(b), Some(c), Some(d)) => {
                assert_eq!(a, c, "a == c");
                assert_eq!(b, d, "b == d");
                assert_eq!(Some(&a), x0, "a == x0 (Observed state are immutable)");
                assert_eq!(Some(&b), x1, "b == x1 (Observed state are immutable)");
                assert_eq!(Some(&c), x0, "c == x0 (Observed state are immutable)");
                assert_eq!(Some(&d), x1, "d == x1 (Observed state are immutable)");
            }
        }

        assert!(
            [x0, x1] == [Some(&'a'), Some(&'b')] || [x0, x1] == [Some(&'b'), Some(&'a')],
            "final state is always complete."
        );
    });
}
