
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestRecord {
    pub request_id: usize,
    pub worker_id: usize,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub response_time_ms: f64,
    pub status_code: u16,
    pub success: bool,
    pub error_message: Option<String>,
    pub response_size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    pub test_name: String,
    pub url: String,
    pub method: String,
    pub test_started_at: DateTime<Utc>,
    pub test_finished_at: DateTime<Utc>,
    pub total_duration_secs: f64,
    pub parallel_workers: usize,
    pub total_requests_planned: usize,
    pub total_requests_sent: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub success_rate_percent: f64,
    pub tps_overall: f64,
    pub tps_success: f64,
    pub response_time_min_ms: f64,
    pub response_time_max_ms: f64,
    pub response_time_avg_ms: f64,
    pub response_time_p50_ms: f64,
    pub response_time_p75_ms: f64,
    pub response_time_p90_ms: f64,
    pub response_time_p95_ms: f64,
    pub response_time_p99_ms: f64,
    pub response_time_stddev_ms: f64,
    pub status_code_distribution: String,
}

impl TestSummary {
    pub fn calculate(
        test_name: &str,
        url: &str,
        method: &str,
        started_at: DateTime<Utc>,
        finished_at: DateTime<Utc>,
        parallel_workers: usize,
        total_requests_planned: usize,
        records: &[RequestRecord],
    ) -> Self {
        let total_duration_secs = (finished_at - started_at).num_milliseconds() as f64 / 1000.0;

        let total_sent = records.len();
        let successful: Vec<&RequestRecord> = records.iter().filter(|r| r.success).collect();
        let failed = total_sent - successful.len();
        let success_rate = if total_sent > 0 {
            (successful.len() as f64 / total_sent as f64) * 100.0
        } else {
            0.0
        };

        // TPS
        let tps_overall = if total_duration_secs > 0.0 {
            total_sent as f64 / total_duration_secs
        } else {
            0.0
        };
        let tps_success = if total_duration_secs > 0.0 {
            successful.len() as f64 / total_duration_secs
        } else {
            0.0
        };

        let mut times: Vec<f64> = records.iter().map(|r| r.response_time_ms).collect();
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());

        let (min_rt, max_rt, avg_rt, stddev_rt) = if times.is_empty() {
            (0.0, 0.0, 0.0, 0.0)
        } else {
            let min = times[0];
            let max = *times.last().unwrap();
            let avg = times.iter().sum::<f64>() / times.len() as f64;
            let variance = times.iter().map(|t| (t - avg).powi(2)).sum::<f64>() / times.len() as f64;
            let stddev = variance.sqrt();
            (min, max, avg, stddev)
        };

        let p50 = percentile(&times, 50.0);
        let p75 = percentile(&times, 75.0);
        let p90 = percentile(&times, 90.0);
        let p95 = percentile(&times, 95.0);
        let p99 = percentile(&times, 99.0);

        // Status code distribution
        let mut status_map: std::collections::HashMap<u16, usize> = std::collections::HashMap::new();
        for r in records {
            *status_map.entry(r.status_code).or_insert(0) += 1;
        }
        let mut status_vec: Vec<(u16, usize)> = status_map.into_iter().collect();
        status_vec.sort_by_key(|(code, _)| *code);
        let status_dist = status_vec.iter()
            .map(|(code, count)| format!("{}:{}", code, count))
            .collect::<Vec<_>>()
            .join(", ");

        TestSummary {
            test_name: test_name.to_string(),
            url: url.to_string(),
            method: method.to_string(),
            test_started_at: started_at,
            test_finished_at: finished_at,
            total_duration_secs,
            parallel_workers,
            total_requests_planned,
            total_requests_sent: total_sent,
            successful_requests: successful.len(),
            failed_requests: failed,
            success_rate_percent: success_rate,
            tps_overall,
            tps_success,
            response_time_min_ms: min_rt,
            response_time_max_ms: max_rt,
            response_time_avg_ms: avg_rt,
            response_time_p50_ms: p50,
            response_time_p75_ms: p75,
            response_time_p90_ms: p90,
            response_time_p95_ms: p95,
            response_time_p99_ms: p99,
            response_time_stddev_ms: stddev_rt,
            status_code_distribution: status_dist,
        }
    }
}

fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let index = (p / 100.0) * (sorted.len() - 1) as f64;
    let lower = index.floor() as usize;
    let upper = index.ceil() as usize;
    if lower == upper {
        sorted[lower]
    } else {
        let frac = index - lower as f64;
        sorted[lower] * (1.0 - frac) + sorted[upper] * frac
    }
}
