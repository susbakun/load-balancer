#[derive(Debug)]
pub struct Server {
    pub address: String,
    pub(super) is_alive: bool,
}

impl Server {
    pub fn new(address: String) -> Self {
        Self {
            address,
            is_alive: true,
        }
    }
}
