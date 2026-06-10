use tokio::time::Instant;

use super::*;

pub async fn set_healthcheck_interval(
    healthcheck_config: HealthCheckConfig,
    pool: Arc<Mutex<Pool>>,
) {
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

pub async fn calculate_latency(ip_addr: &str) -> Result<Duration> {
    let start = Instant::now();

    let mut stream = TcpStream::connect(ip_addr).await?;

    stream.write_all(b"ping").await?;
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    let latency = start.elapsed();

    Ok(latency)
}
