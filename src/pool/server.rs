use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::Relaxed};

use crate::BackendConfig;

#[derive(Debug)]
pub struct Server {
    pub address: String,
    pub(super) is_alive: AtomicBool,
    pub(super) weight: AtomicU32,
    pub active_requests: AtomicUsize,
}

impl Server {
    pub fn new(address: String) -> Self {
        Self {
            address,
            is_alive: AtomicBool::new(true),
            weight: AtomicU32::new(1),
            active_requests: AtomicUsize::new(0),
        }
    }

    pub fn set_weight(&self, weight: f32) {
        self.weight.store(weight.to_bits(), Relaxed);
    }

    pub fn get_weight(&self) -> f32 {
        f32::from_bits(self.weight.load(Relaxed))
    }

    pub fn track_active_request(&self) -> impl Drop + '_ {
        self.active_requests.fetch_add(1, Relaxed);

        scopeguard::guard(self, |backend| {
            backend.active_requests.fetch_sub(1, Relaxed);
        })
    }
}

impl From<BackendConfig> for Server {
    fn from(backend_config: BackendConfig) -> Self {
        Self::new(backend_config.address)
    }
}
