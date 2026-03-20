FROM rust:1.75-slim as builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release --package metrics-exporter

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libpq5 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home appuser

WORKDIR /home/appuser

COPY --from=builder /app/target/release/metrics-exporter .

EXPOSE 8080

USER appuser

ENTRYPOINT ["./metrics-exporter"]
