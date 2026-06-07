# load-balancer

A small TCP load balancer written in Rust. It accepts client connections, forwards raw bytes to a healthy backend, and returns the backend response to the client.

This project is a learning exercise in networking: TCP listeners, connection pooling, health checks, and YAML-based configuration.

## Features

- **TCP proxy** — reads a client request and forwards it to a backend over TCP
- **Backend pool** — multiple backends defined in a config file
- **Periodic health checks** — probes backends on an interval and marks unreachable ones as unhealthy
- **YAML configuration** — listen address, backends, and health-check settings without recompiling

## Requirements

- [Rust](https://rust.rust-lang.org/) (2024 edition)
- GNU netcat (`nc`) or any TCP backend for local testing

On macOS with Homebrew, netcat is typically GNU netcat:

```bash
nc -h   # should show "GNU netcat"
```

## Quick start

### 1. Configure backends

Edit `configs.yaml`:

```yaml
listen:
  address: "127.0.0.1:8085"

algorithm: "round_robin"

health_check:
  interval_seconds: 5
  timeout_seconds: 2

backends:
  - address: "127.0.0.1:9000"
  - address: "127.0.0.1:9001"
  - address: "127.0.0.1:9002"
```

### 2. Start fake HTTP backends (for testing)

Use a loop so each backend accepts more than one connection. **Use `-p` with GNU netcat:**

**Terminal 1 — backend on 9000:**

```bash
while true; do
  printf 'HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\nHello from 9000' | nc -l -p 9000
done
```

**Terminal 2 — backend on 9001:**

```bash
while true; do
  printf 'HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\nHello from 9001' | nc -l -p 9001
done
```

> **Note:** Avoid port `5000` on macOS — it is often used by AirPlay Receiver.

### 3. Run the load balancer

```bash
cargo run
```

The process reads `configs.yaml` from the **current working directory**, so run it from the project root.

### 4. Send a request through the load balancer

The load balancer waits for the client to send bytes before forwarding. Plain `nc` with no input will hang.

```bash
printf 'GET / HTTP/1.1\r\n\r\n' | nc 127.0.0.1 8085
```

Or use curl:

```bash
curl http://127.0.0.1:8085/
```

## Configuration reference

| Field | Description |
|-------|-------------|
| `listen.address` | Address the load balancer binds to (e.g. `127.0.0.1:8085`) |
| `health_check.interval_seconds` | How often to probe each backend |
| `health_check.timeout_seconds` | TCP connect timeout for each probe |
| `backends[].address` | Backend host/port (e.g. `127.0.0.1:9000`) |
| `algorithm` | Reserved for future use (`round_robin` is not wired up yet) |

## How it works

```
Client                    Load Balancer                  Backend
  |                            |                            |
  |---- TCP connect ---------->|                            |
  |---- request bytes -------->|                            |
  |                            |---- TCP connect ---------->|
  |                            |---- forward request ------>|
  |                            |<---- response bytes -------|
  |<---- response bytes -------|                            |
```

1. Bind a `TcpListener` on the configured address.
2. Spawn a background task that runs health checks every N seconds.
3. For each accepted client connection:
   - Read up to 4096 bytes from the client.
   - Pick a healthy backend from the pool.
   - Open a TCP connection to that backend, send the request, read the response.
   - Write the backend response back to the client.

Health checks use a simple TCP connect probe — they do not send HTTP requests.

## Project layout

```
load-balancer/
├── configs.yaml       # Runtime configuration
├── Cargo.toml
└── src/
    ├── main.rs        # Entry point, accept loop, request forwarding
    ├── config.rs      # Config structs (serde)
    ├── constants.rs
    └── pool/
        ├── mod.rs     # Pool, server selection, health checks
        └── server.rs  # Backend server state
```

## Development

Build:

```bash
cargo build
```

Run tests:

```bash
cargo test
```

Run with debug output (health-check logs go to stdout/stderr):

```bash
cargo run
```

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| `Connection refused` on startup | A client connected but no backend is listening on the configured port |
| Client hangs, nothing printed | Client connected but sent no data; use `printf 'GET / ...'` or `curl` |
| `nc -l -p 9001` exits immediately | GNU netcat handles one connection per invocation; wrap it in `while true` |
| Backend never receives traffic | Health check may consume a one-shot `nc` listener; use a loop |
| Port 5000 behaves oddly on macOS | AirPlay Receiver often owns port 5000; use 9000+ instead |

## Current limitations

- **Raw TCP proxy** — no HTTP parsing; bytes are forwarded as-is. Backends must speak whatever protocol the client sends.
- **Single-threaded request handling** — each client is handled sequentially in the accept loop (no per-connection tasks yet).
- **Health check side effects** — a TCP probe counts as a connection; one-shot `nc` backends will exit after a probe.
- **`algorithm` in config** — not implemented; server selection uses the first healthy backend from the current pool index.
- **Dead backends** — a failed health check sets `is_alive = false`; a later successful probe does not currently flip it back to `true`.
