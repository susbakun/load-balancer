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
    pub fn new(servers: Vec<Server>) -> Self {
        Self {
            servers,
            next_available_ind: 0,
        }
    }

    pub fn next_server(&mut self) -> Option<&Server> {
        for (ind, server) in self.servers.iter().enumerate() {
            if server.is_alive {
                self.next_available_ind = ind;
                return Some(server);
            }
        }

        None
    }

    pub fn test_servers(&mut self) {
        for server in self.servers.iter_mut() {
            let target_address = &server.address.parse().unwrap();

            match TcpStream::connect_timeout(target_address, THREE_SECS) {
                Ok(_) => println!("Port is open: {target_address}"),
                Err(err) => {
                    server.is_alive = false;
                    eprintln!("port is closed: {target_address} - {err}");
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_health() {
        let mut pool = Pool::new(vec![
            Server::new("127.0.0.1:9000".into()),
            Server::new("127.0.0.1:9001".into()),
        ]);

        pool.test_servers();

        println!("{pool:?}")
    }
}
