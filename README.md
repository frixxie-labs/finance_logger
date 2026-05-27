# finance_logger

A CLI tool that fetches stock/financial ticker data from Yahoo Finance and logs it as measurements to a [hemrs](http://hemrs/) REST API.

For each ticker symbol, it registers a device and sensors on the hemrs platform, then posts timestamped price, volume, open, high, low, and change measurements.

## Requirements

- Rust (edition 2024)
- `protobuf-compiler` system package

## Build

```bash
cargo build --release --locked
```

## Run

```bash
# Default: fetches AAPL, posts to http://hemrs/
finance_logger

# Custom symbols and hemrs URL
finance_logger AAPL MSFT EQNR.OL --hemrs-url http://localhost:8080 --log-level debug
```

### CLI Arguments

| Argument | Default | Description |
|---|---|---|
| `SYMBOL` (positional, repeatable) | `AAPL` | Stock ticker symbols to fetch |
| `--hemrs-url` | `http://hemrs/` | Base URL of the hemrs REST API |
| `--log-level` | `info` | Log level: trace, debug, info, warn, error |

## Test

```bash
cargo test --locked --verbose
```

## Lint

```bash
cargo fmt --all --check
cargo clippy --locked --all-targets --all-features -- -D warnings
```

## Docker

```bash
docker build -t finance_logger .
docker run finance_logger AAPL MSFT --hemrs-url http://your-hemrs-host/
```

## Project Structure

```
src/
├── main.rs              # Entry point and CLI orchestration
├── finance/             # Yahoo Finance data fetching
├── device/              # hemrs device registration
├── sensor/              # hemrs sensor registration (with cache)
└── measurement/         # hemrs measurement posting
```

Each module follows a trait + client + types pattern with property-based tests.
