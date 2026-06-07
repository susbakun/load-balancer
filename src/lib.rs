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

async fn set_healthcheck_interval(healthcheck_config: HealthCheckConfig, pool: Arc<Mutex<Pool>>) {
    let period = Duration::from_secs(healthcheck_config.interval_seconds);
    let mut interval = time::interval(period);
    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let mut pool_gaurd = pool.lock().await;
            pool_gaurd.test_servers(&healthcheck_config);
        }
    });
}

async fn setup_listener(pool: Arc<Mutex<Pool>>, listen_address: String) -> Result<()> {
    let listener = TcpListener::bind(listen_address).await?;
    while let Ok((mut client_stream, _)) = listener.accept().await {
        let pool_cloned = Arc::clone(&pool);
        if let Err(_) = direct_request(pool_cloned, &mut client_stream).await {
            continue;
        }
    }
    Ok(())
}

async fn direct_request(pool: Arc<Mutex<Pool>>, client_stream: &mut TcpStream) -> Result<()> {
    let request = read_request(client_stream).await?;

    let backend_address = {
        let mut pool = pool.lock().await;
        pool.next_server()
            .map(|server| server.address.clone())
            .ok_or_else(|| anyhow!("couldn't find a healthy server"))?
    };

    let response = proxy_request(&backend_address, &request).await?;
    client_stream
        .write_all(&response)
        .await
        .map_err(|err| anyhow!("Write error: {err}"))
}

async fn read_request(client_stream: &mut TcpStream) -> Result<Vec<u8>> {
    let mut buf = [0; 4096];
    let n = client_stream.read(&mut buf).await?;
    Ok(buf[..n].to_vec())
}

async fn proxy_request(backend_address: &str, request: &[u8]) -> Result<Vec<u8>> {
    let mut backend_stream = TcpStream::connect(backend_address).await?;
    backend_stream.write_all(request).await?;
    backend_stream.shutdown().await?;

    let mut response = Vec::new();
    backend_stream.read_to_end(&mut response).await?;
    Ok(response)
}
