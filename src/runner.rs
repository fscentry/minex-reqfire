use std::sync::Arc;
use tokio::sync::{Semaphore, Mutex};
use chrono::Utc;
use tracing::info;

use crate::config::TestConfig;
use crate::http_client::ApiClient;
use crate::models::{RequestRecord, TestSummary};
use crate::body_loader::load_bodies_from_file;

pub type ProgressCallback = Arc<dyn Fn(usize, usize) + Send + Sync>;

pub struct TestRunner {
    config: TestConfig,
}

impl TestRunner {
    pub fn new(config: TestConfig) -> Self {
        TestRunner { config }
    }

    pub async fn run(
        &self,
        on_progress: Option<ProgressCallback>,
    ) -> Result<(Vec<RequestRecord>, TestSummary), Box<dyn std::error::Error>> {

        // Load bodies from file if body_file not empty
        // bodies = Vec<Option<serde_json::Value>>
        //   - not empty: length vec = total row, every item = Some(json)
        //   - empty: vec empty, every request use static body from config
        let bodies: Vec<Option<serde_json::Value>> = if let Some(ref path) = self.config.body_file {
            let loaded = load_bodies_from_file(path)?;
            loaded.into_iter().map(Some).collect()
        } else {
            Vec::new()
        };

        let total_requests = if !bodies.is_empty() {
            bodies.len()
        } else {
            self.config.total_requests
        };

        info!("═══════════════════════════════════════════════════");
        info!("  🚀 Start  : {}", self.config.test_name);
        info!("  URL       : {}", self.config.url);
        info!("  Method    : {}", self.config.method);
        info!("  Workers   : {}", self.config.parallel_limit);
        info!("  Requests  : {}", total_requests);
        if self.config.body_file.is_some() {
            info!("  Body Src  : file ({} rows)", total_requests);
        } else if self.config.body.is_some() {
            info!("  Body Src  : static JSON from config");
        } else {
            info!("  Body Src  : (no have body)");
        }
        info!("═══════════════════════════════════════════════════");

        // Build HTTP client (shared antar worker)
        let client = Arc::new(
            ApiClient::new(&self.config)
                .map_err(|e| format!("Failed Create HTTP client: {}", e))?
        );

        // Semaphore limitation concurrent requests
        let semaphore = Arc::new(Semaphore::new(self.config.parallel_limit));
        // Shared vector result
        let results: Arc<Mutex<Vec<RequestRecord>>> = Arc::new(Mutex::new(
            Vec::with_capacity(total_requests)
        ));

        let completed = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let test_started = Utc::now();
        let bodies = Arc::new(bodies);

        // Spawn all tasks
        let mut handles = Vec::with_capacity(total_requests);

        for request_idx in 0..total_requests {
            let client_clone   = Arc::clone(&client);
            let sem_clone      = Arc::clone(&semaphore);
            let results_clone  = Arc::clone(&results);
            let completed_clone = Arc::clone(&completed);
            let bodies_clone   = Arc::clone(&bodies);
            let progress_cb    = on_progress.clone();
            let parallel_limit = self.config.parallel_limit;

            let worker_id = request_idx % parallel_limit;

            let handle = tokio::spawn(async move {
                let _permit = sem_clone.acquire().await.unwrap();

                let body_override = if !bodies_clone.is_empty() {
                    bodies_clone.get(request_idx).and_then(|b| b.clone())
                } else {
                    None
                };

                let record = client_clone
                    .execute_request(request_idx + 1, worker_id + 1, body_override)
                    .await;

                // save result
                {
                    let mut locked = results_clone.lock().await;
                    locked.push(record);
                }

                // Update & report progress
                let done = completed_clone.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                if let Some(cb) = progress_cb {
                    cb(done, total_requests);
                }
            });

            handles.push(handle);
        }

        // await all
        for handle in handles {
            handle.await.map_err(|e| format!("Task panic: {}", e))?;
        }

        let test_finished = Utc::now();

        let mut records = Arc::try_unwrap(results)
            .map_err(|_| "Gagal unwrap results Arc")?
            .into_inner();
        records.sort_by_key(|r| r.request_id);

        let summary = TestSummary::calculate(
            &self.config.test_name,
            &self.config.url,
            &self.config.method.to_string(),
            test_started,
            test_finished,
            self.config.parallel_limit,
            total_requests,
            &records,
        );

        self.print_summary_console(&summary);

        Ok((records, summary))
    }

    /// Print summary ke console
    fn print_summary_console(&self, s: &TestSummary) {
        info!("");
        info!("╔══════════════════════════════════════════════════╗");
        info!("║              📊 TEST SUMMARY RESULT              ║");
        info!("╠══════════════════════════════════════════════════╣");
        info!("║  Test        : {:<34}║", s.test_name);
        info!("║  Duration    : {:<.3} detik{:<28} ║", s.total_duration_secs, "");
        info!("╠══════════════════════════════════════════════════╣");
        info!("║  Total Req   : {:<34} ║", s.total_requests_sent);
        info!("║  Success     : {:<34} ║", s.successful_requests);
        info!("║  Failed      : {:<34} ║", s.failed_requests);
        info!("║  Success Rate: {:<.2}%{:<31} ║", s.success_rate_percent, "");
        info!("╠══════════════════════════════════════════════════╣");
        info!("║  TPS Overall : {:<.2} req/s{:<28} ║", s.tps_overall, "");
        info!("║  TPS Success : {:<.2} req/s{:<28} ║", s.tps_success, "");
        info!("╠══════════════════════════════════════════════════╣");
        info!("║  Response Time (ms):                             ║");
        info!("║    Min    : {:<38} ║", format!("{:.2}", s.response_time_min_ms));
        info!("║    Max    : {:<38} ║", format!("{:.2}", s.response_time_max_ms));
        info!("║    Avg    : {:<38} ║", format!("{:.2}", s.response_time_avg_ms));
        info!("║    P50    : {:<38} ║", format!("{:.2}", s.response_time_p50_ms));
        info!("║    P75    : {:<38} ║", format!("{:.2}", s.response_time_p75_ms));
        info!("║    P90    : {:<38} ║", format!("{:.2}", s.response_time_p90_ms));
        info!("║    P95    : {:<38} ║", format!("{:.2}", s.response_time_p95_ms));
        info!("║    P99    : {:<38} ║", format!("{:.2}", s.response_time_p99_ms));
        info!("║    StdDev : {:<38} ║", format!("{:.2}", s.response_time_stddev_ms));
        info!("╠══════════════════════════════════════════════════╣");
        info!("║  Status Codes: {:<33} ║", s.status_code_distribution);
        info!("╚══════════════════════════════════════════════════╝");
    }
}