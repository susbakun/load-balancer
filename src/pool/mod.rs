use super::*;

mod server;
pub use server::*;
use tokio::time::{Instant, timeout};

#[derive(Debug)]
pub struct Pool {
    servers: Vec<Server>,
    next_available_ind: usize,
}

impl Pool {
    pub fn new(servers: Vec<BackendConfig>) -> Self {
        let servers = servers
            .into_iter()
            .map(|server| server.into())
            .collect::<Vec<Server>>();

        Self {
            servers,
            next_available_ind: 0,
        }
    }

    pub fn next_server(&mut self, algorithm: &str) -> Option<&Server> {
        if self.servers.is_empty() {
            return None;
        }

        return match algorithm {
            "round_robin" => self.round_robin(),
            "weighted_round_robin" => self.round_robin(),
            _ => None,
        };
    }

    fn round_robin(&mut self) -> Option<&Server> {
        let n = self.servers.len();
        for _ in 0..n {
            let idx = self.next_available_ind;
            self.next_available_ind = (self.next_available_ind + 1) % n;
            if self.servers[idx].is_alive {
                return Some(&self.servers[idx]);
            }
        }
        None
    }

    fn weighted_round_robin(&mut self) -> Option<&Server> {
        todo!()
    }

    pub async fn test_servers(&mut self, healthcheck_config: &HealthCheckConfig) -> Result<()> {
        let time_out = Duration::from_secs(healthcheck_config.timeout_seconds);
        for server in self.servers.iter_mut() {
            let target_address = &server.address;
            let connect_future = TcpStream::connect(target_address);

            match timeout(time_out, connect_future).await? {
                Ok(mut stream) => {
                    let latency = calculate_latency(&mut stream).await?;
                    server.weight = 1.0 / latency;

                    server.is_alive = true;
                    println!("Port is open: {target_address}, latency: {latency}");
                }
                Err(err) => {
                    server.weight = 0.0;
                    server.is_alive = false;
                    eprintln!("port is closed: {target_address} - {err}");
                }
            }
        }
        Ok(())
    }
}

pub async fn calculate_latency(stream: &mut TcpStream) -> Result<f32> {
    let start = Instant::now();

    stream.write_all(b"ping").await?;
    let mut buf = [0u8; 4];
    stream.read_exact(&mut buf).await?;

    let latency = start.elapsed().as_micros() as f32;

    Ok(latency)
}
