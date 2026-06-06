use std::io::{Read, Write};

use anyhow::Result;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

struct Server {
    address: String,
    is_alive: bool,
}

impl Server {
    fn new(address: String) -> Self {
        Self {
            address,
            is_alive: true,
        }
    }
}

struct Pool {
    servers: Vec<Server>,
    next_available_ind: usize,
}

impl Pool {
    fn new(servers: Vec<Server>) -> Self {
        Self {
            servers,
            next_available_ind: 0,
        }
    }

    fn next_server(&mut self) -> Option<&Server> {
        for (ind, server) in self.servers.iter().enumerate() {
            if server.is_alive {
                self.next_available_ind = ind;
                return Some(server);
            }
        }

        None
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let listener = TcpListener::bind("127.0.0.1:8085").await?;
    let mut pool = Pool::new(vec![
        Server::new("127.0.0.1:9000".into()),
        Server::new("127.0.0.1:9001".into()),
    ]);

    while let Ok((mut client_stream, _)) = listener.accept().await {
        let mut buf = [0; 4096];
        let n = client_stream.read(&mut buf).await?;

        if let Some(next_server) = pool.next_server() {
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

            if let Err(err) = client_stream.write_all(&response_bytes).await {
                eprintln!("Write error: {err}");
                continue;
            }
        }
        continue;
    }

    Ok(())
}
