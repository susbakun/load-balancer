use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub listen: ListenConfig,
    pub health_check: HealthCheckConfig,
    pub backends: Vec<BackendConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ListenConfig {
    pub address: String,
}

#[derive(Debug, Deserialize)]
pub struct HealthCheckConfig {
    pub interval_seconds: u64,
    pub timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
pub struct BackendConfig {
    pub address: String,
}
