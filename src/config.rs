use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use clap::Parser;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub url: String,
    pub method: HttpMethod,
    pub body: Option<serde_json::Value>,
    pub body_file: Option<String>,
    pub headers: HashMap<String, String>,
    pub parallel_limit: usize,
    pub total_requests: usize,
    pub timeout_ms: u64,

    #[serde(default)]
    pub danger_accept_invalid_certs: bool,

    #[serde(default = "default_true")]
    pub follow_redirects: bool,

    pub csv_output: String,
    pub summary_output: String,
    pub test_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
    PATCH,
    DELETE,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpMethod::GET    => write!(f, "GET"),
            HttpMethod::POST   => write!(f, "POST"),
            HttpMethod::PUT    => write!(f, "PUT"),
            HttpMethod::PATCH  => write!(f, "PATCH"),
            HttpMethod::DELETE => write!(f, "DELETE"),
        }
    }
}

impl std::str::FromStr for HttpMethod {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET"    => Ok(HttpMethod::GET),
            "POST"   => Ok(HttpMethod::POST),
            "PUT"    => Ok(HttpMethod::PUT),
            "PATCH"  => Ok(HttpMethod::PATCH),
            "DELETE" => Ok(HttpMethod::DELETE),
            _        => Err(format!("Method Not Found: {}", s)),
        }
    }
}

/* CLI Arguments using Clap */
#[derive(Parser, Debug)]
#[command(
    name = "api_perf_test",
    about = "🚀 API Performance Testing Tool - Testing TPS & Response Time",
    long_about = "Tool for testing API performance with parallel request.\nProduce Report CSV detail dan summary TPS.",
    version = "1.0.0"
)]
pub struct CliArgs {
    #[arg(short = 'u', long, help = "URL API endpoint (will be test)")]
    pub url: Option<String>,

    #[arg(
        short = 'm',
        long,
        default_value = "GET",
        help = "HTTP method: GET, POST, PUT, PATCH, DELETE"
    )]
    pub method: String,

    #[arg(
        short = 'b',
        long,
        help = "Body JSON (exp: '{\"key\":\"value\"}')"
    )]
    pub body: Option<String>,

    #[arg(
        short = 'f',
        long,
        help = "Path ke file .txt — each line is JSON body for 1 request"
    )]
    pub body_file: Option<String>,

    #[arg(
        short = 'H',
        long,
        help = "Custom headers format: 'Authorization:Bearer token,Content-Type:application/json'"
    )]
    pub headers: Option<String>,

    #[arg(
        short = 'p',
        long,
        default_value = "8",
        help = "total  parallel process (concurrent workers)"
    )]
    pub parallel: usize,

    #[arg(
        short = 'n',
        long,
        default_value = "100",
        help = "Total size request will be sent"
    )]
    pub requests: usize,

    #[arg(
        short = 't',
        long,
        default_value = "30000",
        help = "Timeout per request "
    )]
    pub timeout: u64,

    #[arg(
        short = 'k',
        long,
        default_value = "false",
        help = "Skip verification TLS certificate (for self-signed cert / staging)"
    )]
    pub insecure: bool,

    #[arg(
        long,
        default_value = "false",
        help = "disable auto-follow redirect"
    )]
    pub no_redirect: bool,

    #[arg(
        long,
        default_value = "results.csv",
        help = "Name file output CSV for result details"
    )]
    pub csv_output: String,

    /// Output summary filename
    #[arg(
        long,
        default_value = "summary.txt",
        help = "Name file output for summary result"
    )]
    pub summary_output: String,

    #[arg(
        long,
        default_value = "API Performance Test",
        help = "Label/name for this test"
    )]
    pub test_name: String,

    #[arg(
        short = 'c',
        long,
        help = "Path to file configuration JSON (if used, override all argument)"
    )]
    pub config_file: Option<String>,
}

fn default_true() -> bool { true }

impl TestConfig {
    pub fn from_cli(args: &CliArgs) -> Result<Self, Box<dyn std::error::Error>> {
        let method = args.method.parse::<HttpMethod>()
            .map_err(|e| format!("Error parsing method: {}", e))?;

        let body = if let Some(body_str) = &args.body {
            Some(serde_json::from_str(body_str)
                .map_err(|e| format!("Error parsing JSON body: {}", e))?)
        } else {
            None
        };

        let headers = parse_headers(args.headers.as_deref())?;

        let url = args.url.clone()
            .ok_or("--url should be entered if not used --config-file")?;

        Ok(TestConfig {
            url,
            method,
            body,
            body_file: args.body_file.clone(),
            headers,
            parallel_limit: args.parallel,
            total_requests: args.requests,
            timeout_ms: args.timeout,
            danger_accept_invalid_certs: args.insecure,
            follow_redirects: !args.no_redirect,
            csv_output: args.csv_output.clone(),
            summary_output: args.summary_output.clone(),
            test_name: args.test_name.clone(),
        })
    }

    pub fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed reading config file '{}': {}", path, e))?;
        let config: TestConfig = serde_json::from_str(&content)
            .map_err(|e| format!("Failed parse config JSON: {}", e))?;
        Ok(config)
    }
}

fn parse_headers(header_str: Option<&str>) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
    let mut map = HashMap::new();
    if let Some(s) = header_str {
        for pair in s.split(',') {
            let pair = pair.trim();
            if pair.is_empty() { continue; }
            let parts: Vec<&str> = pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(format!("Format header is wrong: '{}'. use :  'Key:Value'", pair).into());
            }
            map.insert(parts[0].trim().to_string(), parts[1].trim().to_string());
        }
    }
    Ok(map)
}


impl Default for TestConfig {
    fn default() -> Self {
        TestConfig {
            url: "https://httpbin.org/post".to_string(),
            method: HttpMethod::POST,
            body: Some(serde_json::json!({
                "test": "api_performance",
                "timestamp": "auto"
            })),
            body_file: None,
            headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h.insert("Accept".to_string(), "application/json".to_string());
                h
            },
            parallel_limit: 8,
            total_requests: 100,
            timeout_ms: 30_000,
            danger_accept_invalid_certs: false,
            follow_redirects: true,
            csv_output: "results.csv".to_string(),
            summary_output: "summary.txt".to_string(),
            test_name: "Default API Test".to_string(),
        }
    }
}