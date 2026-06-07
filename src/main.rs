#![allow(dead_code)]
use std::{fs::File, sync::Arc, time::Duration};

use anyhow::{Result, anyhow};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
    sync::Mutex,
    time,
};

pub use crate::constants::*;

mod constants;
mod pool;
use pool::*;
mod config;
pub use config::*;

#[tokio::main]
async fn main() -> Result<()> {
    let file = File::open("configs.yaml")?;
    let config: Config = serde_yaml::from_reader(file)?;

    let pool = Pool::new(config.backends);
    let pool = Arc::new(Mutex::new(pool));

    let mut interval = time::interval(Duration::from_secs(config.health_check.interval_seconds));

    let pool_cloned = Arc::clone(&pool);
    let healthcheck_config = config.health_check;

    tokio::spawn(async move {
        loop {
            interval.tick().await;
            let mut pool_gaurd = pool_cloned.lock().await;
            pool_gaurd.test_servers(&healthcheck_config);
        }
    });

    let listener = TcpListener::bind(config.listen.address).await?;
    while let Ok((mut client_stream, _)) = listener.accept().await {
        let pool_cloned = Arc::clone(&pool);
        if let Err(_) = direct_request(pool_cloned, &mut client_stream).await {
            continue;
        }
    }

    Ok(())
}

async fn direct_request(pool: Arc<Mutex<Pool>>, client_stream: &mut TcpStream) -> Result<()> {
    let mut pool_gaurd = pool.lock().await;

    let mut buf = [0; 4096];
    let n = client_stream.read(&mut buf).await?;

    if let Some(next_server) = pool_gaurd.next_server() {
        let address = &next_server.address;
        let mut sever_stream = TcpStream::connect(address).await?;
        let mut response_bytes = Vec::new();

        sever_stream.write_all(&buf[..n]).await?;
        sever_stream.shutdown().await?;
        let mut temp = [0; 4096];

        loop {
            let bytes_read = sever_stream.read(&mut temp).await?;
            if bytes_read == 0 {
                break;
            }

            response_bytes.extend_from_slice(&temp[..bytes_read]);
        }

        return match client_stream.write_all(&response_bytes).await {
            Ok(_) => Ok(()),
            Err(err) => Err(anyhow!("Write error: {err}")),
        };
    }

    Err(anyhow!("couldn't find a healthy server"))
}
