# 🚀 Mini Experiment - API Performance Test Tool (Rust)

A modular tool to measure API performance in parallel — producing **TPS**, **response time statistics**, and exporting to **CSV + Summary**.

---

## 📁 Module Structure

```
api_perf_test/
├── Cargo.toml              ← Dependencies
├── config_example.json     ← Example JSON configuration
├── README.md
└── src/
    ├── main.rs             ← Entry point & orchestration
    ├── config.rs           ← Configuration & CLI args (TestConfig, CliArgs)
    ├── models.rs           ← Data structs (RequestRecord, TestSummary)
    ├── http_client.rs      ← HTTP client & per-request timing
    ├── runner.rs           ← Parallel engine based on semaphore
    └── exporter.rs         ← CSV & summary file export
```

---

## ⚙️ Prerequisites

- Rust 1.70+ → install via https://rustup.rs
- Internet connection (to download dependencies during the first build)

---

## 🔨 Build

```bash
# Debug build (for development)
cargo build

# Release build (RECOMMENDED for testing — faster)
cargo build --release

# Binary location:
# ./target/release/api_perf_test  (Linux/Mac)
# ./target/release/api_perf_test.exe  (Windows)
```

---

## 🚀 Usage

### 1. Via CLI Arguments

```bash
# Simple GET request example
./target/release/api_perf_test \
  --url "https://httpbin.org/get" \
  --method GET \
  --parallel 10 \
  --requests 100

# POST example with JSON body and custom headers
./target/release/api_perf_test \
  --url "https://your-api.com/api/login" \
  --method POST \
  --body '{"username":"admin","password":"secret"}' \
  --headers "Authorization:Bearer TOKEN123,X-App-Id:myapp" \
  --parallel 8 \
  --requests 200 \
  --timeout 5000 \
  --csv-output "output/results.csv" \
  --summary-output "output/summary.txt" \
  --test-name "Login API Test"
```

### 2. Via JSON Configuration File

```bash
# Edit config_example.json as needed, then:
./target/release/api_perf_test --config-file config_example.json
```

### 3. All CLI Options

```
Options:
  -u, --url            API endpoint URL (REQUIRED)
  -m, --method         HTTP method: GET/POST/PUT/PATCH/DELETE [default: GET]
  -b, --body           JSON body as string
  -H, --headers        Custom headers: "Key1:Val1,Key2:Val2"
  -p, --parallel       Number of parallel workers [default: 8]
  -n, --requests       Total number of requests [default: 100]
  -t, --timeout        Timeout per request (ms) [default: 30000]
      --csv-output     CSV output file name [default: results.csv]
      --summary-output Summary file name [default: summary.txt]
      --test-name      Label for this test
  -c, --config-file    Path to JSON configuration file
  -h, --help           Show help
  -V, --version        Show version
```

---

## 📊 Output

### CSV File (`results.csv`)
Each row = one request:

| Column | Description |
|--------|------------|
| `request_id` | Request sequence number |
| `worker_id` | Worker ID that executed the request |
| `started_at` | Start timestamp (ISO 8601 ms) |
| `finished_at` | End timestamp |
| `response_time_ms` | Response duration (ms, 4 decimals) |
| `status_code` | HTTP status code (0 = network error) |
| `success` | `true` if status is 2xx |
| `response_size_bytes` | Response body size |
| `error_message` | Error message (if failed) |

### Summary File (`summary.txt`)
Contains a readable report:
- Test info (URL, method, start/end time)
- Total duration, success/failure count, success rate
- **TPS Overall** and **TPS Success**
- Response time: Min, Max, Avg, StdDev, P50, P75, P90, P95, P99
- Status code distribution
- JSON configuration used

### Summary JSON (`summary.json`)
JSON format of the summary — suitable for CI/CD integration, dashboards, or monitoring systems.

---

## 🔢 Parallel Execution Mechanism

```
Total 200 requests, parallel_limit = 8

Semaphore(8)
│
├── Worker 1 ──► request #1
├── Worker 2 ──► request #2
├── Worker 3 ──► request #3
├── Worker 4 ──► request #4
├── Worker 5 ──► request #5
├── Worker 6 ──► request #6
├── Worker 7 ──► request #7
└── Worker 8 ──► request #8
                    │
                    ▼ (when one finishes, slot opens)
               request #9, #10, etc...
```

Uses `tokio::sync::Semaphore` to precisely limit the number of concurrent requests.

---

## 📈 TPS Interpretation

- **TPS Overall** = `total_requests / total_duration_seconds`
- **TPS Success** = `successful_requests / total_duration_seconds`

Example: 200 requests in 25 seconds → **8 TPS**

---

## 🛠️ Environment Variables

```bash
# Set log level (default: info)
RUST_LOG=debug ./target/release/api_perf_test ...
RUST_LOG=warn  ./target/release/api_perf_test ...   # minimal output
```

---

## 💡 Tips

1. **Use release build** for optimal performance
2. **Start with small parallelism** (4–8), then increase to find the limit
3. **Pay attention to P95/P99** — more informative than average for SLA
4. **CSV can be imported** into Excel/Google Sheets for further analysis  