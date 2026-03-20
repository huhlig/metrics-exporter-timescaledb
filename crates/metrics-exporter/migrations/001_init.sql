-- Create hypertable for metrics
CREATE TABLE IF NOT EXISTS metrics (
    id BIGSERIAL,
    name TEXT NOT NULL,
    value JSONB NOT NULL,
    labels JSONB NOT NULL DEFAULT '{}',
    timestamp TIMESTAMPTZ NOT NULL,
    PRIMARY KEY (id, timestamp)
);

SELECT create_hypertable('metrics', 'timestamp', chunk_time_interval => INTERVAL '1 day');

CREATE INDEX IF NOT EXISTS idx_metrics_name ON metrics (name);
CREATE INDEX IF NOT EXISTS idx_metrics_name_timestamp ON metrics (name, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_metrics_labels ON metrics USING GIN (labels);

ALTER TABLE metrics SET (
    timescaledb.compression,
    timescaledb.compress_segmentby = 'name'
);

SELECT add_compression_policy('metrics', INTERVAL '1 hour');

-- Continuous aggregate for 1-minute rollups
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1m
WITH (timescaledb.continuous) AS
SELECT name,
       time_bucket('1 minute', timestamp) AS bucket,
       labels,
       (value->>'type')::text AS metric_type,
       CASE
           WHEN value->>'type' = 'Gauge' THEN AVG((value->>'value')::numeric)
           WHEN value->>'type' = 'Counter' THEN SUM((value->>'value')::bigint)::numeric
           ELSE NULL
       END AS avg_value,
       MIN(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS min_value,
       MAX(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS max_value,
       COUNT(*) AS count
FROM metrics
GROUP BY name, bucket, labels, metric_type;

SELECT add_continuous_aggregate_policy('metrics_1m',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '1 minute',
    schedule_interval => INTERVAL '1 minute');

-- Continuous aggregate for 5-minute rollups
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_5m
WITH (timescaledb.continuous) AS
SELECT name,
       time_bucket('5 minutes', timestamp) AS bucket,
       labels,
       (value->>'type')::text AS metric_type,
       CASE
           WHEN value->>'type' = 'Gauge' THEN AVG((value->>'value')::numeric)
           WHEN value->>'type' = 'Counter' THEN SUM((value->>'value')::bigint)::numeric
           ELSE NULL
       END AS avg_value,
       MIN(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS min_value,
       MAX(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS max_value,
       COUNT(*) AS count
FROM metrics
GROUP BY name, bucket, labels, metric_type;

SELECT add_continuous_aggregate_policy('metrics_5m',
    start_offset => INTERVAL '1 hour',
    end_offset => INTERVAL '5 minutes',
    schedule_interval => INTERVAL '5 minutes');

-- Continuous aggregate for 1-hour rollups
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1h
WITH (timescaledb.continuous) AS
SELECT name,
       time_bucket('1 hour', timestamp) AS bucket,
       labels,
       (value->>'type')::text AS metric_type,
       CASE
           WHEN value->>'type' = 'Gauge' THEN AVG((value->>'value')::numeric)
           WHEN value->>'type' = 'Counter' THEN SUM((value->>'value')::bigint)::numeric
           ELSE NULL
       END AS avg_value,
       MIN(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS min_value,
       MAX(
           CASE
               WHEN value->>'type' = 'Gauge' THEN (value->>'value')::numeric
               WHEN value->>'type' = 'Counter' THEN (value->>'value')::bigint::numeric
               ELSE NULL
           END
       ) AS max_value,
       COUNT(*) AS count
FROM metrics
GROUP BY name, bucket, labels, metric_type;

SELECT add_continuous_aggregate_policy('metrics_1h',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- Retention policy (disabled by default, uncomment to enable)
-- SELECT add_retention_policy('metrics', INTERVAL '90 days');
