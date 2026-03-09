# pi-vitals

A lightweight real-time system monitoring dashboard for Raspberry Pi, built with Rust.

Streams live CPU, memory, disk, network, process, and power telemetry from the Pi to a browser via WebSocket.

![Dashboard](https://img.shields.io/badge/platform-Raspberry%20Pi-red) ![Language](https://img.shields.io/badge/language-Rust-orange) ![License](https://img.shields.io/badge/license-MIT-blue)

## Features

- **CPU** — usage %, frequency, core count, temperature
- **Memory & Swap** — total / used / available in MB with usage %
- **Disk** — space per mount point (GB, usage %)
- **Network** — per-interface RX/TX bytes and live KB/s rates
- **Processes** — top 10 by CPU usage (PID, name, CPU %, memory %)
- **Power** (Pi 4/5) — core voltage, throttling status, estimated watt draw, hourly/monthly energy
- **System** — hostname and uptime
- Updates every **2 seconds** over WebSocket; auto-reconnects on drop

## Stack

| Layer | Technology |
|-------|-----------|
| Web server | [Axum](https://github.com/tokio-rs/axum) 0.7 |
| Async runtime | [Tokio](https://tokio.rs) |
| System info | [sysinfo](https://github.com/GuillaumeGomez/sysinfo) 0.30 |
| Serialization | serde / serde_json |
| Frontend | Vanilla HTML/CSS/JS (embedded in binary) |

## Requirements

- Raspberry Pi (tested on Pi 3 B+, Pi 4, Pi 5)
- Rust toolchain + `aarch64-unknown-linux-gnu` cross-compile target (for deployment from macOS/Linux)
- `cross` or `cargo` with a suitable linker configured for the target

## Build & Run Locally (on the Pi)

```bash
git clone https://github.com/YOUR_USERNAME/pi-vitals
cd pi-vitals
cargo build --release
./target/release/pi-vitals
```

Open `http://<pi-hostname>:3000` in your browser.

## Cross-Compile & Deploy from macOS/Linux

Set your Pi credentials then run the deploy script:

```bash
export PI_USER=your-pi-username
export PI_HOST=raspberrypi.local   # or the Pi's IP address

./deploy.sh
```

The script will:
1. Cross-compile for `aarch64-unknown-linux-gnu`
2. `scp` the binary to `~/pi-vitals/` on the Pi
3. Print the SSH command to start it

## Configuration

The server binds to `0.0.0.0:3000` by default. There is no built-in authentication — deploy behind a firewall or add a reverse proxy with auth if you expose the port externally.

## License

MIT
