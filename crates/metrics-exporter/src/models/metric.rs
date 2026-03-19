use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Label {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    #[serde(flatten)]
    pub value: MetricValue,
    pub labels: HashMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum MetricValue {
    Gauge(f64),
    Counter(i64),
    Histogram(HistogramValue),
    Summary(SummaryValue),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramValue {
    pub sum: f64,
    pub count: u64,
    pub bounds: Vec<f64>,
    pub counts: Vec<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SummaryValue {
    pub quantiles: Vec<f64>,
    pub values: Vec<f64>,
}
