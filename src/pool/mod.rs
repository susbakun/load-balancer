use std::sync::atomic::Ordering::Relaxed;

use super::*;

mod server;
use rand::distr::{Distribution, weighted::WeightedIndex};
pub use server::*;
use tokio::time::{Instant, timeout};

#[derive(Debug)]
pub struct Pool {
    pub servers: Vec<Arc<Server>>,
    next_available_ind: usize,
}

impl Pool {
    pub fn new(servers: Vec<BackendConfig>) -> Self {
        let servers = servers
            .into_iter()
            .map(|server| Arc::new(server.into()))
            .collect::<Vec<Arc<Server>>>();

        Self {
            servers,
            next_available_ind: 0,
        }
    }

    pub fn next_server(&mut self, algorithm: &str) -> Option<Arc<Server>> {
        if self.servers.is_empty() {
            return None;
        }

        return match algorithm {
            "round_robin" => self.round_robin(),
            "weighted_round_robin" => self.weighted_round_robin(),
            "least_connections" => self.least_connection(),
            _ => None,
        };
    }

    fn round_robin(&mut self) -> Option<Arc<Server>> {
        let n = self.servers.len();
        for _ in 0..n {
            let idx = self.next_available_ind;
            self.next_available_ind = (self.next_available_ind + 1) % n;
            let is_alive = self.servers[idx].get_is_alive();

            if is_alive {
                return Some(self.servers[idx].clone());
            }
        }
        None
    }

    fn weighted_round_robin(&mut self) -> Option<Arc<Server>> {
        let weights = self
            .servers
            .iter()
            .filter(|server| server.get_is_alive())
            .map(|server| server.get_weight())
            .filter(|weight| *weight != 0.0)
            .collect::<Vec<f32>>();

        let Ok(dist) = WeightedIndex::new(weights) else {
            return None;
        };

        let mut rng = rand::rng();

        let index = dist.sample(&mut rng);

        self.next_available_ind = index;

        Some(self.servers[index].clone())
    }

    fn least_connection(&mut self) -> Option<Arc<Server>> {
        let mut selected = None;
        let mut least_connection_count = usize::MAX;

        for server in &self.servers {
            if !server.get_is_alive() {
                continue;
            }

            let active_requests = server.active_requests.load(Relaxed);

            if active_requests < least_connection_count {
                least_connection_count = active_requests;
                selected = Some(server.clone());
            }
        }

        selected
    }

    pub async fn test_servers(&mut self, healthcheck_config: &HealthCheckConfig) -> Result<()> {
        let time_out = Duration::from_secs(healthcheck_config.timeout_seconds);
        for server in self.servers.iter_mut() {
            let target_address = &server.address.clone();
            let connect_future = TcpStream::connect(target_address);

            match timeout(time_out, connect_future).await? {
                Ok(mut stream) => {
                    let latency = Self::calculate_latency(&mut stream).await?;
                    server.set_weight(1.0 / latency);
                    server.set_is_alive(true);

                    println!("Port is open: {target_address}, latency: {latency}");
                }
                Err(err) => {
                    server.set_weight(0.0);
                    server.set_is_alive(false);

                    eprintln!("port is closed: {target_address} - {err}");
                }
            }
        }
        Ok(())
    }

    async fn calculate_latency(stream: &mut TcpStream) -> Result<f32> {
        let start = Instant::now();

        stream.write_all(b"ping").await?;
        let mut buf = [0u8; 4];
        stream.read_exact(&mut buf).await?;

        let latency = start.elapsed().as_micros() as f32;

        Ok(latency)
    }
}
