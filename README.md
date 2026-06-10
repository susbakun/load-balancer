# load-balancer

A small HTTP load balancer written in Rust. It accepts client connections, picks a healthy backend, proxies the HTTP request, and returns the backend response.

This project is a learning exercise in networking: TCP listeners, HTTP proxying with Hyper, backend pools, latency-aware routing, health checks, and YAML-based configuration.

## Features

- **HTTP/1.1 proxy** — parses client HTTP requests and forwards them to backends via Hyper
- **Round robin** — distributes requests evenly across healthy backends
- **Weighted round robin** — routes more traffic to faster backends using latency-derived weights
- **Backend pool** — multiple backends defined in a config file
- **Periodic health checks** — probes backends on an interval, measures latency, updates weights, and marks unreachable backends as unhealthy
- **Concurrent connections** — each client connection is handled in its own async task
- **YAML configuration** — listen address, algorithm, backends, and health-check settings without recompiling

## Requirements

- [Rust](https://rust.rust-lang.org/) (2024 edition)
- HTTP-capable backends for local testing (see below)

For quick tests with fake backends, GNU netcat works for basic HTTP responses:

```bash
nc -h   # should show "GNU netcat"
```

## Quick start

### 1. Configure backends

Edit `configs.yaml`:

```yaml
listen:
  address: "127.0.0.1:8085"

algorithm: "round_robin"   # or "weighted_round_robin"

health_check:
  interval_seconds: 5
  timeout_seconds: 2

backends:
  - address: "127.0.0.1:9000"
  - address: "127.0.0.1:9001"
  - address: "127.0.0.1:9002"
```

Supported algorithms:

| Value | Behavior |
|-------|----------|
| `round_robin` | Rotate through healthy backends one at a time |
| `weighted_round_robin` | Pick a backend at random, weighted by `1 / latency` |

### 2. Start backends (for testing)

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

For `weighted_round_robin`, health checks send a custom `ping` probe and expect 4 bytes back — plain `nc` HTTP backends do not support that. Use real HTTP servers instead, for example:

```bash
python3 -m http.server 9000
python3 -m http.server 9001
```

> **Note:** Avoid port `5000` on macOS — it is often used by AirPlay Receiver.

### 3. Run the load balancer

```bash
cargo run
```

The process reads `configs.yaml` from the **current working directory**, so run it from the project root.

### 4. Send requests

```bash
curl http://127.0.0.1:8085/
```

Send several requests to observe routing:

- **`round_robin`** — responses alternate between healthy backends (e.g. `Hello from 9000`, `Hello from 9001`, …)
- **`weighted_round_robin`** — faster backends receive more requests; check load balancer logs for `selected ...` output

## Configuration reference

| Field | Description |
|-------|-------------|
| `listen.address` | Address the load balancer binds to (e.g. `127.0.0.1:8085`) |
| `algorithm` | `"round_robin"` or `"weighted_round_robin"` |
| `health_check.interval_seconds` | How often to probe each backend |
| `health_check.timeout_seconds` | Connect + latency probe timeout per backend |
| `backends[].address` | Backend host/port (e.g. `127.0.0.1:9000`) |

## How it works

```
Client                    Load Balancer                  Backend
  |                            |                            |
  |---- HTTP request --------->|                            |
  |                            |---- pick backend --------->|
  |                            |---- HTTP proxy ----------->|
  |                            |<---- HTTP response --------|
  |<---- HTTP response --------|                            |
```

1. Load `configs.yaml` and validate the algorithm.
2. Build a backend pool and spawn a background health-check task.
3. Bind a `TcpListener` and accept client connections concurrently (one task per connection).
4. For each HTTP request on a client connection:
   - Pick a backend using the configured algorithm.
   - Open an HTTP/1.1 client connection to that backend.
   - Forward the request and return the response to the client.

### Health checks and weighting

On each health-check interval, for every backend the pool:

1. Attempts a TCP connect (with timeout).
2. On success, runs a latency probe (`ping` + 4-byte read).
3. Sets `weight = 1 / latency` (latency in microseconds).
4. Sets `is_alive = true`.

On failure, `weight = 0` and `is_alive = false`.

For `weighted_round_robin`, backends with higher weight (lower latency) are chosen more often.

## Project layout

```
load-balancer/
├── configs.yaml       # Runtime configuration
├── Cargo.toml
└── src/
    ├── main.rs        # Entry point (calls load_balancer::run)
    ├── lib.rs         # App setup: config, pool, health checks, listener
    ├── config.rs      # Config structs (serde)
    ├── constants.rs   # Supported algorithm names
    ├── types.rs       # Shared types (BoxBody)
    ├── healthcheck.rs # Periodic backend probing scheduler
    ├── request.rs     # Accept loop, Hyper server/client proxy
    └── pool/
        ├── mod.rs     # Pool, routing algorithms, latency + health checks
        └── server.rs  # Backend state (address, alive, weight)
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

Run with debug output (health-check and weighted-selection logs go to stdout/stderr):

```bash
cargo run
```

## Troubleshooting

| Symptom | Likely cause |
|---------|----------------|
| `unknown algorithm` on startup | `algorithm` is not `round_robin` or `weighted_round_robin` |
| `502 Bad Gateway` | Selected backend refused the connection or the HTTP proxy failed |
| `Connection refused` | No healthy backend is listening on the configured port |
| `nc -l -p 9001` exits immediately | GNU netcat handles one connection per invocation; wrap it in `while true` |
| Backend never receives traffic | Health check consumed a one-shot `nc` listener; use a loop |
| Weighted mode marks all backends dead | `nc` backends do not respond to the `ping` latency probe; use real HTTP servers |
| Same backend every time (round robin) | Only one backend is healthy |
| Port 5000 behaves oddly on macOS | AirPlay Receiver often owns port 5000; use 9000+ instead |

## Current limitations

- **HTTP/1.1 only** — no HTTP/2 or HTTPS termination.
- **Response buffering** — backend response bodies are collected in memory before being sent to the client (not a streaming proxy).
- **Two algorithms only** — `round_robin` and `weighted_round_robin`; no least-connections or sticky sessions.
- **Custom latency probe** — health checks expect backends to respond to a raw `ping` write with 4 bytes; this does not work with simple `nc` HTTP fakes.
- **Health check side effects** — a TCP probe counts as a connection; one-shot `nc` backends will exit after a probe.
- **No connect retry** — if the selected backend refuses the proxy connection, the client gets `502` instead of trying the next healthy backend.
- **Backend picked per connection** — the backend address is chosen once when the client connects, not per request on keep-alive connections.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright (c) 2026 AmirSaeed AryanMehr
