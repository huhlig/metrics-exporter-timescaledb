# Metrics Exporter TimescaleDB

A self-contained metrics exporter backed by TimescaleDB with automatic downsampling, compression, and a built-in visualization dashboard.

## Features

- **TimescaleDB Backend**: Leverages TimescaleDB hypertables for efficient time-series storage
- **Automatic Downsampling**: Continuous aggregates at 1m, 5m, and 1h intervals
- **Compression**: Automatic chunk compression for reduced storage
- **REST API**: Query metrics, time-series data, and aggregated statistics
- **Built-in Dashboard**: Web UI with Chart.js visualization - no external tools needed

## Quick Start

### Using Docker Compose

```bash
# Start services
docker-compose up -d

# View logs
docker-compose logs -f app

# Stop services
docker-compose down
```

Access the dashboard at `http://localhost:8080`

### Manual Setup

```bash
# Install Rust dependencies
cargo build --release

# Set environment
export DATABASE_URL=postgres://postgres:password@localhost:5432/metrics
export RUST_LOG=info

# Run the server
cargo run --release
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /` | Dashboard UI |
| `GET /api/health` | Health check |
| `GET /api/metrics` | List all metric names |
| `GET /api/metrics/{name}` | Get metric details |
| `GET /api/metrics/{name}/timeseries` | Get time-series data |
| `GET /api/metrics/{name}/aggregated` | Get aggregated data |

### Query Parameters

- `start` - Start time (ISO 8601)
- `end` - End time (ISO 8601)
- `interval` - Aggregation interval (`1m`, `5m`, `1h`)

### Example

```bash
# List available metrics
curl http://localhost:8080/api/metrics

# Get aggregated data for the last 24 hours
curl "http://localhost:8080/api/metrics/my_metric/aggregated?interval=1h"
```

## Architecture

```
┌─────────────┐     ┌─────────────────┐     ┌──────────────┐
│ Your App    │────▶│  Metrics Exporter│────▶│  TimescaleDB │
└─────────────┘     └─────────────────┘     └──────────────┘
                           │                      │
                           ▼                      ▼
                    ┌─────────────┐        ┌─────────────────┐
                    │  Dashboard  │        │ Continuous      │
                    │  (Chart.js) │        │ Aggregates      │
                    └─────────────┘        │ (1m/5m/1h)     │
                                          └─────────────────┘
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://postgres:password@localhost:5432/metrics` |
| `RUST_LOG` | Logging level | `info` |

### Docker Environment

```bash
# .env file
TIMESCALEDB_PASSWORD=your_secure_password
```

## Development

### Prerequisites

- Rust 1.75+
- PostgreSQL 17+ with TimescaleDB extension
- Docker (for integration tests)

### Running Tests

```bash
# Run all tests
cargo test --workspace

# Run with TimescaleDB
TEST_DATABASE_URL=postgres://postgres:password@localhost:5431/metrics_test cargo test

# Run clippy
cargo clippy --workspace --all-targets -- -D warnings

# Format code
cargo fmt --all
```

### Using the Test Script

```bash
# Start TimescaleDB container for testing
./scripts/test-timescale.sh

# Run tests against container
./scripts/test-timescale.sh --test

# On Windows
.\scripts\test-timescale.ps1 --test
```

## Docker Services

The docker-compose setup includes:

| Service | Port | Description |
|---------|------|-------------|
| `timescaledb` | 5432 | TimescaleDB database |
| `app` | 8080 | Metrics exporter API with dashboard |

### Default Credentials

- **PostgreSQL**: `postgres` / `password`

## Metric Data Model

Metrics are stored with the following structure:

```rust
struct Metric {
    id: Option<i64>,
    name: String,
    value: MetricValue,  // Gauge, Counter, Histogram, Summary
    labels: HashMap<String, String>,
    timestamp: DateTime<Utc>,
}
```

### Supported Metric Types

- **Gauge**: Floating-point value (`f64`)
- **Counter**: Integer value (`i64`)
- **Histogram**: Sum, count, bounds, and bucket counts
- **Summary**: Quantiles and values

## Database Schema

The `001_init.sql` migration creates:

- `metrics` - Main hypertable
- `metrics_1m` - 1-minute continuous aggregate
- `metrics_5m` - 5-minute continuous aggregate
- `metrics_1h` - 1-hour continuous aggregate

## License

MIT License - see LICENSE file for details
