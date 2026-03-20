#Requires -RunAsAdministrator

$ErrorActionPreference = "Stop"

$ContainerName = "timescale-test-$(Get-Random)"
$TimescaleImage = "timescale/timescaledb:latest-pg17"
$PostgresPassword = "testpassword"
$DbName = "metrics_test"

function Cleanup {
    Write-Host "Cleaning up container..."
    docker rm -f $ContainerName 2>$null | Out-Null
}

Register-EngineEvent -Action { Cleanup } -EventName PowerShell.Exiting

Write-Host "Starting TimescaleDB container..."
docker run -d `
    --name $ContainerName `
    -e POSTGRES_PASSWORD="$PostgresPassword" `
    -e POSTGRES_USER="postgres" `
    -p 5431:5432 `
    $TimescaleImage | Out-Null

Write-Host "Waiting for TimescaleDB to be ready..."
$maxRetries = 30
$retryCount = 0

while ($retryCount -lt $maxRetries) {
    $result = docker exec $ContainerName pg_isready -U postgres 2>$null
    if ($LASTEXITCODE -eq 0) { break }
    $retryCount++
    Write-Host "Waiting for TimescaleDB... ($retryCount/$maxRetries)"
    Start-Sleep -Seconds 2
}

if ($retryCount -ge $maxRetries) {
    Write-Error "Timeout waiting for TimescaleDB to start"
    exit 1
}

Write-Host "Creating test database..."
docker exec $ContainerName psql -U postgres -c "DROP DATABASE IF EXISTS $DbName;" 2>$null | Out-Null
docker exec $ContainerName psql -U postgres -c "CREATE DATABASE $DbName;"

Write-Host "Verifying TimescaleDB extension..."
docker exec $ContainerName psql -U postgres -d $DbName -c "CREATE EXTENSION IF NOT EXISTS timescaledb;"

Write-Host "Running migrations..."
Get-Content "crates\metrics-exporter\migrations\001_init.sql" | docker exec -i $ContainerName psql -U postgres -d $DbName

Write-Host "Verifying hypertable..."
docker exec $ContainerName psql -U postgres -d $DbName -c "SELECT hypertable_name, num_chunks FROM timescaledb_information.hypertables WHERE hypertable_name = 'metrics';"

Write-Host "Verifying continuous aggregates..."
docker exec $ContainerName psql -U postgres -d $DbName -c "SELECT view_name, view_owner FROM timescaledb_information.continuous_aggregates;"

Write-Host ""
Write-Host "TimescaleDB is ready for testing!"
Write-Host "Connection: postgres://postgres:$PostgresPassword@localhost:5431/$DbName"
Write-Host ""

if ($Args -contains "--test") {
    Write-Host "Running cargo tests..."
    $env:DATABASE_URL = "postgres://postgres:$PostgresPassword@localhost:5432/$DbName"
    cargo test --package metrics-exporter
}
