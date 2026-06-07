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
        let end = self.next_available_ind;
        let mut current_ind = (self.next_available_ind + 1) % n;

        // checking servers one by one except
        // current one
        while current_ind != end {
            let server = &self.servers[current_ind];
            if server.is_alive {
                self.next_available_ind = current_ind;
                return Some(server);
            }

            current_ind = (current_ind + 1) % n;
        }

        // check the current server if
        // there wasn't any other available
        // around it

        if self.servers[current_ind].is_alive {
            return Some(&self.servers[current_ind]);
        }

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
