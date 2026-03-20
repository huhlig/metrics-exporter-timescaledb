#!/bin/bash
set -e

CONTAINER_NAME="timescale-test-$$"
TIMESCALE_IMAGE="timescale/timescaledb:latest-pg17"
POSTGRES_PASSWORD="testpassword"
DB_NAME="metrics_test"

cleanup() {
    echo "Cleaning up container..."
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
}

trap cleanup EXIT

echo "Starting TimescaleDB container..."
docker run -d \
    --name "$CONTAINER_NAME" \
    -e POSTGRES_PASSWORD="$POSTGRES_PASSWORD" \
    -e POSTGRES_USER="postgres" \
    -p 5431:5432 \
    "$TIMESCALE_IMAGE"

echo "Waiting for TimescaleDB to be ready..."
MAX_RETRIES=30
RETRY_COUNT=0
until docker exec "$CONTAINER_NAME" pg_isready -U postgres >/dev/null 2>&1; do
    RETRY_COUNT=$((RETRY_COUNT + 1))
    if [ $RETRY_COUNT -ge $MAX_RETRIES ]; then
        echo "Timeout waiting for TimescaleDB to start"
        exit 1
    fi
    echo "Waiting for TimescaleDB... ($RETRY_COUNT/$MAX_RETRIES)"
    sleep 2
done

echo "Creating test database..."
docker exec "$CONTAINER_NAME" psql -U postgres -c "DROP DATABASE IF EXISTS $DB_NAME;" 2>/dev/null || true
docker exec "$CONTAINER_NAME" psql -U postgres -c "CREATE DATABASE $DB_NAME;"

echo "Verifying TimescaleDB extension..."
docker exec "$CONTAINER_NAME" psql -U postgres -d "$DB_NAME" -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"

echo "Running migrations..."
docker exec -i "$CONTAINER_NAME" psql -U postgres -d "$DB_NAME" < crates/metrics-exporter/migrations/001_init.sql

echo "Verifying hypertable..."
docker exec "$CONTAINER_NAME" psql -U postgres -d "$DB_NAME" -c "SELECT hypertable_name, num_chunks FROM timescaledb_information.hypertables WHERE hypertable_name = 'metrics';"

echo "Verifying continuous aggregates..."
docker exec "$CONTAINER_NAME" psql -U postgres -d "$DB_NAME" -c "SELECT view_name, view_owner FROM timescaledb_information.continuous_aggregates;"

echo ""
echo "TimescaleDB is ready for testing!"
echo "Connection: postgres://postgres:$POSTGRES_PASSWORD@localhost:5431/$DB_NAME"
echo ""

if [ "$1" = "--test" ]; then
    echo "Running cargo tests..."
    DATABASE_URL="postgres://postgres:$POSTGRES_PASSWORD@localhost:5432/$DB_NAME" \
        cargo test --package metrics-exporter
fi
