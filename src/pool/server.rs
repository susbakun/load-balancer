use crate::BackendConfig;

#[derive(Debug)]
pub struct Server {
    pub address: String,
    pub(super) is_alive: bool,
    pub(super) weight: f32,
}

impl Server {
    pub fn new(address: String) -> Self {
        Self {
            address,
            is_alive: true,
            weight: 1.0,
        }
    }
}

impl From<BackendConfig> for Server {
    fn from(value: BackendConfig) -> Self {
        Self {
            address: value.address,
            is_alive: true,
            weight: 1.0,
        }
    }
}
