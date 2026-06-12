use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering::Relaxed};

use crate::{BackendConfig, constants::ALPHA};

#[derive(Debug)]
pub struct Server {
    pub address: String,
    is_alive: AtomicBool,
    weight: AtomicU32,
    pub(super) active_requests: AtomicUsize,
}

impl Server {
    pub fn new(address: String) -> Self {
        Self {
            address,
            is_alive: AtomicBool::new(true),
            weight: AtomicU32::new(1.0f32.to_bits()),
            active_requests: AtomicUsize::new(0),
        }
    }

    pub fn update_weight(&self, new_latency: f32, algorithm: &String) {
        // we first check if the server is available
        // if so we check for the alogrithm on update
        // it based on that
        let new_weight = if new_latency == 0.0 {
            0.0
        } else if algorithm == "PEWMA" {
            let old_weight = self.get_weight();

            let ewma_latency = if old_weight == 0.0 {
                new_latency
            } else {
                let old_latency = 1.0 / old_weight;
                (ALPHA * new_latency) + ((1.0 - ALPHA) * old_latency)
            };

            1.0 / ewma_latency
        } else {
            1.0 / new_latency
        };

        self.weight.store(new_weight.to_bits(), Relaxed);
    }

    pub fn get_weight(&self) -> f32 {
        f32::from_bits(self.weight.load(Relaxed))
    }

    pub fn set_is_alive(&self, is_alive: bool) {
        self.is_alive.store(is_alive, Relaxed);
    }

    pub fn get_is_alive(&self) -> bool {
        self.is_alive.load(Relaxed)
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
