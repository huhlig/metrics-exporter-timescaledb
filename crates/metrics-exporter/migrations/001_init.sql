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
CREATE INDEX IF NOT EXISTS idx_metrics_labels ON metrics USING GIN (labels);

-- Continuous aggregate for 1-minute rollups
CREATE MATERIALIZED VIEW IF NOT EXISTS metrics_1m
WITH (timescaledb.continuous) AS
SELECT name,
       time_bucket('1 minute', timestamp) AS bucket,
       labels,
       jsonb_object_agg(label_key, label_value) AS label_values,
       AVG((value->>'value')::numeric) AS avg_value,
       MIN((value->>'value')::numeric) AS min_value,
       MAX((value->>'value')::numeric) AS max_value,
       COUNT(*) AS count
FROM (
    SELECT name, timestamp, labels,
           jsonb_object_keys(labels) AS label_key,
           jsonb_array_elements(jsonb_extract_path(value, 'bounds')) AS label_value,
           value
    FROM metrics
) sub
GROUP BY name, bucket, labels;

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
       AVG((value->>'value')::numeric) AS avg_value,
       MIN((value->>'value')::numeric) AS min_value,
       MAX((value->>'value')::numeric) AS max_value,
       COUNT(*) AS count
FROM metrics
GROUP BY name, bucket, labels;

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
       AVG((value->>'value')::numeric) AS avg_value,
       MIN((value->>'value')::numeric) AS min_value,
       MAX((value->>'value')::numeric) AS max_value,
       COUNT(*) AS count
FROM metrics
GROUP BY name, bucket, labels;

SELECT add_continuous_aggregate_policy('metrics_1h',
    start_offset => INTERVAL '1 day',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour');

-- Retention policy (optional, uncomment to enable)
-- SELECT add_retention_policy('metrics', INTERVAL '90 days');
