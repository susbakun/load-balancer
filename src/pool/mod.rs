use std::net::TcpStream;

use super::*;

mod server;
pub use server::*;

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

    pub fn next_server(&mut self) -> Option<&Server> {
        let n = self.servers.len();
        let mut current_ind = self.next_available_ind;

        while current_ind < n {
            let server = &self.servers[current_ind];
            if server.is_alive {
                return Some(server);
            }

            current_ind += 1;
        }

        self.next_available_ind = current_ind + 1;

        None
    }

    pub fn test_servers(&mut self, healthcheck_config: &HealthCheckConfig) {
        let time_out = Duration::from_secs(healthcheck_config.timeout_seconds);
        for server in self.servers.iter_mut() {
            let target_address = &server.address.parse().unwrap();

            match TcpStream::connect_timeout(target_address, time_out) {
                Ok(_) => println!("Port is open: {target_address}"),
                Err(err) => {
                    server.is_alive = false;
                    eprintln!("port is closed: {target_address} - {err}");
                }
            }
        }
    }
}
