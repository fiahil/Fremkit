#[cfg(test)]
mod test {
    use super::*;

    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_notifier() {
        let (nx, rx) = Notifier::new();

        let h = thread::spawn(move || {
            rx.wait();
        });

        thread::sleep(Duration::from_millis(100));

        nx.notify();

        assert!(h.join().is_ok());
    }

    #[test]
    fn test_broadcast() {
        let (a, b) = Notifier::new();
        let c = a.clone();

        let h1 = thread::spawn(move || {
            b.wait();
        });
        let h2 = thread::spawn(move || {
            c.wait();
        });

        thread::sleep(Duration::from_millis(100));

        a.notify();

        assert!(h1.join().is_ok());
        assert!(h2.join().is_ok());
    }
}
