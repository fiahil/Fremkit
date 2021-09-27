// use canal::Canal;

/// An Aqueduc is a collection of Canals. It is the main entry point for
/// creating Canals and spawning threads.
#[derive(Debug, Clone)]
pub struct Aqueduc {}

impl Aqueduc {
    pub fn new() -> Self {
        Aqueduc {}
    }
}

impl Default for Aqueduc {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn init() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    fn test_aqueduc() {
        init();

        let _ = Aqueduc::new();
    }
}
