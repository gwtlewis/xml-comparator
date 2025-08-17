# Performance Testing Framework

This directory contains a comprehensive performance testing suite for the XML-Compare-API, designed to validate performance under realistic workloads and identify bottlenecks.

## 📁 Directory Structure

```
perf/
├── tools/                    # Payload generation tools
│   ├── Cargo.toml           # Dependencies for tools
│   └── gen_payload.rs       # XML payload generator
├── scripts/                 # Test execution scripts
│   ├── k6_batch_test.js     # k6 load testing script
│   ├── wrk_batch.lua        # wrk2 Lua script for HTTP testing
│   ├── micro_benchmark.rs   # Component-level benchmarks
│   └── run_perf_tests.sh    # Main test orchestrator
├── results/                 # Test outputs (auto-generated)
└── README.md               # This file
```

## 🎯 Test Scenarios

### 1. **Smoke Test** (100 pairs)
- **Purpose**: Quick sanity check
- **Duration**: ~1 minute
- **Pass Criteria**: 95% requests < 60s

### 2. **Nominal Load** (100,000 pairs)
- **Purpose**: Primary target workload
- **Duration**: ~5-15 minutes
- **Pass Criteria**: 95% requests < 300s, Memory < 8GB

### 3. **Soak Test** (6 × 100,000 pairs)
- **Purpose**: Detect memory leaks and degradation
- **Duration**: ~1 hour
- **Pass Criteria**: No memory growth between batches

### 4. **Stress Test** (Concurrent users)
- **Purpose**: Find breaking point under contention
- **Users**: 1 → 3 → 5 (ramping)
- **Pass Criteria**: No failures under normal load

## 📊 XML Dataset Characteristics

Performance tests use a realistic distribution of XML complexity:

- **60%** Depth-2 XMLs (simple structure)
- **30%** Depth-3 XMLs (moderate complexity)  
- **10%** Depth-5 XMLs (complex nested structure)

Sample XML structures:

```xml
<!-- Depth 2 -->
<level2 id="doc1_2" value="123">
  <level1 id="doc1_1" value="124">doc1_content</level1>
</level2>

<!-- Depth 5 -->
<level5 id="doc1_5" value="123">
  <level4 id="doc1_4" value="124">
    <level3 id="doc1_3" value="125">
      <level2 id="doc1_2" value="126">
        <level1 id="doc1_1" value="127">doc1_content</level1>
      </level2>
    </level3>
  </level4>
</level5>
```

## 🚀 Quick Start

### Prerequisites

Install required tools:

```bash
# Required
cargo install  # Rust toolchain
curl           # HTTP client
jq             # JSON processor

# Optional (for advanced tests)
# k6 - https://k6.io/docs/getting-started/installation/
# wrk - https://github.com/wg/wrk
```

### Run All Tests

```bash
# From project root
./perf/scripts/run_perf_tests.sh
```

This will:
1. Build the API in release mode
2. Start the server on port 3000
3. Run all performance test scenarios
4. Generate a comprehensive report
5. Clean up resources

### Run Individual Tests

```bash
# Payload generator test only
cd perf/tools && cargo run --release --bin gen_payload 1000 > test.json

# k6 tests only (requires k6 installed)
export BASE_URL="http://127.0.0.1:3000"
k6 run perf/scripts/k6_batch_test.js

# wrk tests only (requires wrk installed)  
wrk -t4 -c4 -d60s -s perf/scripts/wrk_batch.lua \
    http://127.0.0.1:3000/xml-compare-api/api/compare/xml/batch

# Micro-benchmarks
cargo run --release perf/scripts/micro_benchmark.rs
```

## 📈 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| **Throughput** | ≥ 333 pairs/sec | 100k pairs in ≤ 300s |
| **Memory Usage** | ≤ 8 GB RSS | Peak memory consumption |
| **CPU Utilization** | ≤ 90% sustained | Average across test |
| **Error Rate** | 0% | Failed HTTP requests |
| **Latency P95** | ≤ 300s | 95th percentile response time |

### Real-world Context

- **100k pairs** ≈ comparing two large enterprise XML datasets
- **300s limit** enables reasonable user wait times
- **Memory target** fits standard cloud VM limits (16GB total)

## 📋 Interpreting Results

### Test Output Locations

All test results are stored in `perf/results/session_YYYYMMDD_HHMMSS/`:

```
session_20240117_143022/
├── performance_report.md   # Executive summary
├── k6_results.json        # Detailed k6 metrics
├── k6_summary.json        # k6 test summary
├── wrk_output.log         # wrk detailed output
├── system_metrics.log     # System resource info
├── server.log             # API server logs
└── payload_test_response.json # Sample API responses
```

### Key Metrics to Monitor

1. **Latency Trends**
   - Look for consistent response times across iterations
   - Watch for degradation in soak tests

2. **Memory Patterns**
   - RSS should stabilize after initial batch
   - No continuous growth indicates memory leaks

3. **Error Rates**
   - Any non-200 responses indicate capacity limits
   - Timeout errors suggest insufficient time limits

4. **Throughput Consistency**
   - Pairs/second should remain stable
   - Significant drops indicate resource contention

### Pass/Fail Determination

Tests automatically fail if:
- Any HTTP request returns non-200 status
- Memory usage exceeds 8GB
- Response time P95 > 300s for nominal load
- System becomes unresponsive

## 🔧 Customization

### Modify Test Parameters

Edit configuration in `run_perf_tests.sh`:

```bash
# Test sizes
SMOKE_SIZE=100
NOMINAL_SIZE=100000    # Adjust for your needs
STRESS_SIZE=50000

# Timeouts
API_TIMEOUT=900s       # 15 minutes
```

### Change XML Complexity

Modify generators in `gen_payload.rs` or `k6_batch_test.js`:

```javascript
function determineDepth(index, total) {
  const percent = (index / total) * 100;
  if (percent < 20) return 7;    // 20% depth 7 (very complex)
  if (percent < 50) return 4;    // 30% depth 4
  return 2;                      // 50% depth 2
}
```

### Add Custom Metrics

Extend k6 script with additional counters:

```javascript
import { Counter } from 'k6/metrics';
const customMetric = new Counter('my_custom_metric');

// In test function
customMetric.add(someValue);
```

## 🐛 Troubleshooting

### Common Issues

**API fails to start**
```bash
# Check if port is in use
lsof -i :3000
# Kill existing process
pkill -f xml-compare-api
```

**Tests timeout**
```bash
# Increase timeouts in scripts
export K6_TIMEOUT=1800s  # 30 minutes
```

**Out of memory**
```bash
# Monitor memory usage
watch -n 5 'ps aux | grep xml-compare-api'
# Reduce batch size
export TEST_SIZE=50000
```

**k6/wrk not found**
```bash
# Install k6
curl https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz -L | tar xvz --strip-components 1

# Install wrk (Ubuntu/Debian)
apt-get install wrk
```

### Performance Debugging

1. **Enable detailed logging**
   ```bash
   RUST_LOG=debug ./target/release/xml-compare-api
   ```

2. **Profile memory usage**
   ```bash
   valgrind --tool=massif ./target/release/xml-compare-api
   ```

3. **CPU profiling**
   ```bash
   perf record -g ./target/release/xml-compare-api
   perf report
   ```

## 🔄 CI/CD Integration

### GitHub Actions Example

```yaml
name: Performance Tests
on:
  schedule:
    - cron: '0 2 * * 1'  # Weekly Monday 2AM
  workflow_dispatch:

jobs:
  perf-test:
    runs-on: ubuntu-latest
    timeout-minutes: 60
    steps:
      - uses: actions/checkout@v4
      - uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
      - name: Install k6
        run: |
          curl https://github.com/grafana/k6/releases/download/v0.48.0/k6-v0.48.0-linux-amd64.tar.gz -L | tar xvz --strip-components 1
          sudo mv k6 /usr/local/bin/
      - name: Run performance tests
        run: ./perf/scripts/run_perf_tests.sh
      - name: Upload results
        uses: actions/upload-artifact@v3
        with:
          name: performance-results
          path: perf/results/
```

This comprehensive performance testing framework ensures the XML-Compare-API can handle real-world workloads reliably and provides early detection of performance regressions.
