/// A Droplet is a single message containing data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Droplet {
    Data,
}

impl Default for Droplet {
    fn default() -> Self {
        Droplet::Data
    }
}
