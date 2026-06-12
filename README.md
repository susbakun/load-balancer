# load-balancer

A small HTTP load balancer written in Rust. It accepts client connections, picks a healthy backend, proxies the HTTP request, and returns the backend response.

This project is a learning exercise in networking: TCP listeners, HTTP proxying with Hyper, backend pools, latency-aware routing, health checks, and YAML-based configuration.

## Features

- **HTTP/1.1 proxy** — parses client HTTP requests and forwards them to backends via Hyper
- **Round robin** — distributes requests evenly across healthy backends
- **Weighted round robin** — routes more traffic to faster backends using `1 / latency` weights
- **Least connections** — sends each request to the healthy backend with the fewest in-flight requests
- **PEWMA** — exponentially weighted moving average of latency; smoother weight updates than a single snapshot
- **Backend pool** — multiple backends defined in a config file
- **Periodic health checks** — probes backends on an interval, measures latency, updates weights, and marks unreachable backends as unhealthy
- **Active request tracking** — counts in-flight proxied requests per backend (used by least connections)
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

algorithm: "round_robin"   # see supported algorithms below

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
| `weighted_round_robin` | Pick a backend at random, weighted by `1 / latency` from the latest health check |
| `least_connections` | Pick the healthy backend with the lowest number of in-flight proxied requests |
| `PEWMA` | Same weighted random selection as above, but latency is smoothed with an EWMA before computing weight |

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

**Terminal 3 — backend on 9002 (optional, for three-backend tests):**

```bash
while true; do
  printf 'HTTP/1.1 200 OK\r\nContent-Length: 14\r\n\r\nHello from 9002' | nc -l -p 9002
done
```

For production-like backends, real HTTP servers also work:

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
- **`weighted_round_robin`** / **`PEWMA`** — faster backends receive more requests over time; check load balancer logs for per-backend latency on each health-check tick
- **`least_connections`** — traffic favors backends with fewer active proxied requests

For scripted tests with `nc`, add `Connection: close` so the client exits cleanly:

```bash
printf 'GET / HTTP/1.1\r\nConnection: close\r\n\r\n' | nc 127.0.0.1 8085
```

## Configuration reference

| Field | Description |
|-------|-------------|
| `listen.address` | Address the load balancer binds to (e.g. `127.0.0.1:8085`) |
| `algorithm` | One of `round_robin`, `weighted_round_robin`, `least_connections`, `PEWMA` |
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
   - Increment that backend's active-request counter for the duration of the proxy.
   - Open an HTTP/1.1 client connection to that backend.
   - Forward the request and return the response to the client.

### Health checks and weighting

On each health-check interval, for every backend the pool:

1. Attempts a TCP connect (with timeout).
2. On success, runs a latency probe (`ping` write + 4-byte read).
3. Updates weight and sets `is_alive = true`.
4. On failure, sets `weight = 0` and `is_alive = false`.

Weight update depends on the algorithm:

| Algorithm | Weight formula |
|-----------|----------------|
| `weighted_round_robin` | `1 / latency` (latency in microseconds from the latest probe) |
| `PEWMA` | `1 / ewma_latency`, where `ewma_latency = α × sample + (1 − α) × old_ewma` (α = `0.2`) |
| `round_robin`, `least_connections` | Weight is still updated on health checks but not used for selection |

For `weighted_round_robin` and `PEWMA`, backends with higher weight (lower latency) are chosen more often via weighted random selection.

### Least connections

Each backend tracks how many requests are currently being proxied. When a request is handled, the counter is incremented at the start of proxying and decremented automatically when the request finishes (including on errors).

## Project layout

```
load-balancer/
├── configs.yaml       # Runtime configuration
├── Cargo.toml
└── src/
    ├── main.rs        # Entry point (calls load_balancer::run)
    ├── lib.rs         # App setup: config, pool, health checks, listener
    ├── config.rs      # Config structs (serde)
    ├── constants.rs   # Supported algorithm names and PEWMA alpha
    ├── types.rs       # Shared types (BoxBody)
    ├── healthcheck.rs # Periodic backend probing scheduler
    ├── request.rs     # Accept loop, Hyper server/client proxy
    └── pool/
        ├── mod.rs     # Pool, routing algorithms, latency + health checks
        └── server.rs  # Backend state (address, alive, weight, active requests)
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
| `unknown algorithm` on startup | `algorithm` is not one of the four supported values |
| `502 Bad Gateway` | Selected backend refused the connection or the HTTP proxy failed |
| `Connection refused` | No healthy backend is listening on the configured port |
| `nc -l -p 9001` exits immediately | GNU netcat handles one connection per invocation; wrap it in `while true` |
| Backend never receives traffic | Health check consumed a one-shot `nc` listener; use a loop |
| Same backend every time (round robin) | Only one backend is healthy |
| `nc` test hangs after first response | HTTP keep-alive — use `Connection: close` in the request or use `curl` |
| Parallel `nc` requests get 502 | All requests may hit the same backend before counters update, or `nc` accepts only one client at a time |
| Port 5000 behaves oddly on macOS | AirPlay Receiver often owns port 5000; use 9000+ instead |

## Current limitations

- **HTTP/1.1 only** — no HTTP/2 or HTTPS termination.
- **Response buffering** — backend response bodies are collected in memory before being sent to the client (not a streaming proxy).
- **Custom latency probe** — health checks send a raw `ping` write and read 4 bytes; this works with `nc` HTTP fakes (response starts with `HTTP`) but is not a real HTTP health endpoint.
- **Health check side effects** — a TCP probe counts as a connection; one-shot `nc` backends will exit after a probe.
- **No connect retry** — if the selected backend refuses the proxy connection, the client gets `502` instead of trying the next healthy backend.
- **Backend picked per connection** — the backend address is chosen once when the client connects, not per request on keep-alive connections.
- **Least connections timing** — the active-request counter is updated when proxying starts, not when the backend is selected; concurrent bursts may still route to the same backend briefly.
- **No sticky sessions** — no client-to-backend affinity.

## License

This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.

Copyright (c) 2026 AmirSaeed AryanMehr
