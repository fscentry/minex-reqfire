mod config;
mod models;
mod http_client;
mod body_loader;
mod runner;
mod exporter;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use clap::Parser;
use tracing::info;
use tracing_subscriber::{fmt, EnvFilter};

use config::{CliArgs, TestConfig};
use runner::TestRunner;
use exporter::{write_csv, write_summary, write_summary_json};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_thread_ids(false)
        .init();

    let args = CliArgs::parse();

    /* Prioritas:
       1. --config-file <path>  → load from file
       2. --url <url> ...       → baca from CLI args
       3. Tidak ada args        → auto-load config_example.json
    */
    let config = if let Some(ref config_path) = args.config_file {
        info!("📂 Reading Configuration from file: {}", config_path);
        TestConfig::from_file(config_path)?
    } else if args.url.is_some() {
        TestConfig::from_cli(&args)?
    } else {
        let default_config = "config_example.json";
        if std::path::Path::new(default_config).exists() {
            info!("📂 Auto-load configuration from: {}", default_config);
            TestConfig::from_file(default_config)?
        } else {
            return Err("Configuration not found!\n\
                 Use This Command Instead :\n\
                 1. cargo run -- --config-file config_example.json\n\
                 2. cargo run -- --url https://... --method POST"
                 .to_string().into());
        }
    };

    /*validation*/
    validate_config(&config)?;

    let config_json = serde_json::to_string_pretty(&config)
        .unwrap_or_else(|_| "{}".to_string());

    let total = config.total_requests;
    let last_reported = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let report_interval = (total / 10).max(1);

    let last_clone = Arc::clone(&last_reported);
    let progress_cb: runner::ProgressCallback = Arc::new(move |done, total| {
        let last = last_clone.load(Ordering::Relaxed);
        if done == total || (done - last) >= report_interval {
            last_clone.store(done, Ordering::Relaxed);
            let pct = (done as f64 / total as f64 * 100.0) as usize;
            let bar = make_progress_bar(pct);
            eprintln!("\r  Progress: [{}] {}/{} ({}%)", bar, done, total, pct);
        }
    });

    eprintln!();
    let runner = TestRunner::new(config.clone());
    let (records, summary) = runner.run(Some(progress_cb)).await?;
    eprintln!();

    // 1. CSV
    write_csv(&records, &config.csv_output)?;
    // 2. Summary text
    write_summary(&summary, &config_json, &config.summary_output)?;
    // 3. Summary JSON
    let json_path = if config.summary_output.ends_with(".txt") {
        config.summary_output.replace(".txt", ".json")
    } else {
        format!("{}.json", config.summary_output)
    };
    write_summary_json(&summary, &json_path)?;

    info!("");
    info!("📁 Output files:");
    info!("   CSV Detail  : {}", config.csv_output);
    info!("   Summary     : {}", config.summary_output);
    info!("   Summary JSON: {}", json_path);
    info!("");
    info!("🏁 Test Finish!");

    Ok(())
}

fn validate_config(config: &TestConfig) -> Result<(), Box<dyn std::error::Error>> {
    if config.url.is_empty() {
        return Err("URL should not be empty".into());
    }
    if !config.url.starts_with("http://") && !config.url.starts_with("https://") {
        return Err(format!("URL not valid (required http:// atau https://): {}", config.url).into());
    }
    if config.parallel_limit == 0 {
        return Err("parallel_limit must >= 1".into());
    }
    // total_requests boleh 0 jika body_file diisi (jumlah ditentukan oleh baris file)
    if config.body_file.is_none() && config.total_requests == 0 {
        return Err("total_requests must >= 1 (or use body file)".into());
    }
    if config.timeout_ms == 0 {
        return Err("timeout_ms must >= 1".into());
    }
    Ok(())
}

fn make_progress_bar(pct: usize) -> String {
    let filled = pct / 5; // 20 chars total
    let empty  = 20 - filled.min(20);
    format!("{}{}", "█".repeat(filled), "░".repeat(empty))
}