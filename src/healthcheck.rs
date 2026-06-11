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
            let mut pool = pool.lock().await;
            let _ = pool.test_servers(&healthcheck_config).await;
        }
    });
}
