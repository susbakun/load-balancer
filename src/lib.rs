#![allow(dead_code)]

use std::{fs::File, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time,
};

mod pool;
use pool::*;
mod config;
pub use config::*;
mod healthcheck;
use healthcheck::*;
mod request;
use request::*;

pub async fn run() -> Result<()> {
    let config = read_config()?;

    // setup the pool of servers
    let pool = setup_pool(config.backends);

    // set an interval for healthchecking the servers
    let pool_cloned = Arc::clone(&pool);
    set_healthcheck_interval(config.health_check, pool_cloned).await;

    // setup tcp listener to provided address
    let pool_cloned = Arc::clone(&pool);
    setup_listener(pool_cloned, config.listen.address).await
}

fn read_config() -> Result<Config> {
    // deserializing the config fiile
    let file = File::open("configs.yaml")?;
    let config: Config = serde_yaml::from_reader(file)?;

    Ok(config)
}

fn setup_pool(backends: Vec<BackendConfig>) -> Arc<Mutex<Pool>> {
    let pool = Pool::new(backends);
    Arc::new(Mutex::new(pool))
}
