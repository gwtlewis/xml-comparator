#!/bin/bash

# Performance Test Runner for XML-Compare-API
# This script orchestrates the complete performance testing pipeline

set -euo pipefail

# Configuration
API_PORT=${APP_PORT:-3000}
API_URL="http://127.0.0.1:${API_PORT}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)/results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TEST_SESSION_DIR="${RESULTS_DIR}/session_${TIMESTAMP}"

# Test parameters
SMOKE_SIZE=${SMOKE_SIZE:-100}
NOMINAL_SIZE=${NOMINAL_SIZE:-100000}
STRESS_SIZE=${STRESS_SIZE:-50000}

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Function to check if a command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# Function to wait for API to be ready
wait_for_api() {
    local max_attempts=30
    local attempt=1
    
    log "Waiting for API to be ready at ${API_URL}..."
    
    while [ $attempt -le $max_attempts ]; do
        if curl -s "${API_URL}/xml-compare-api/health" >/dev/null 2>&1; then
            success "API is ready"
            return 0
        fi
        
        log "Attempt ${attempt}/${max_attempts} - API not ready, waiting 2s..."
        sleep 2
        ((attempt++))
    done
    
    error "API failed to become ready after ${max_attempts} attempts"
    return 1
}

# Function to start the API server
start_api_server() {
    log "Starting XML-Compare-API server..."
    
    # Build in release mode
    log "Building project in release mode..."
    cargo build --release
    
    # Start the server in background
    APP_PORT=$API_PORT ./target/release/xml-compare-api > "${TEST_SESSION_DIR}/server.log" 2>&1 &
    local api_pid=$!
    echo $api_pid > "${TEST_SESSION_DIR}/api.pid"
    
    log "API server started with PID: $api_pid"
    
    # Wait for server to be ready
    wait_for_api
}

# Function to stop the API server
stop_api_server() {
    if [ -f "${TEST_SESSION_DIR}/api.pid" ]; then
        local api_pid=$(cat "${TEST_SESSION_DIR}/api.pid")
        log "Stopping API server (PID: $api_pid)..."
        
        if kill -0 $api_pid 2>/dev/null; then
            kill $api_pid
            sleep 2
            
            # Force kill if still running
            if kill -0 $api_pid 2>/dev/null; then
                warn "Server still running, force killing..."
                kill -9 $api_pid
            fi
        fi
        
        rm -f "${TEST_SESSION_DIR}/api.pid"
        success "API server stopped"
    fi
}

# Function to run k6 tests
run_k6_tests() {
    if ! command_exists k6; then
        warn "k6 not found, skipping k6 tests"
        return 0
    fi
    
    log "Running k6 performance tests..."
    
    local k6_script="$(dirname "$0")/k6_batch_test.js"
    local k6_results="${TEST_SESSION_DIR}/k6_results.json"
    
    # Set environment variables for k6
    export BASE_URL="$API_URL"
    
    # Run k6 with detailed output
    k6 run \
        --out json="${k6_results}" \
        --summary-export="${TEST_SESSION_DIR}/k6_summary.json" \
        "$k6_script" 2>&1 | tee "${TEST_SESSION_DIR}/k6_output.log"
    
    success "k6 tests completed"
}

# Function to run wrk tests
run_wrk_tests() {
    if ! command_exists wrk; then
        warn "wrk not found, skipping wrk tests"
        return 0
    fi
    
    log "Running wrk performance tests..."
    
    local wrk_script="$(dirname "$0")/wrk_batch.lua"
    local wrk_output="${TEST_SESSION_DIR}/wrk_output.log"
    
    # Run wrk with 4 threads, 4 connections, for 5 minutes
    wrk -t4 -c4 -d300s \
        -s "$wrk_script" \
        --latency \
        "${API_URL}/xml-compare-api/api/compare/xml/batch" \
        2>&1 | tee "$wrk_output"
    
    success "wrk tests completed"
}

# Function to run custom Rust payload generator test
run_payload_generator_test() {
    log "Running payload generator test..."
    
    # Ensure results directory exists
    mkdir -p "$TEST_SESSION_DIR"
    
    local payload_file="${TEST_SESSION_DIR}/test_payload_1000.json"
    local response_file="${TEST_SESSION_DIR}/payload_test_response.json"
    local tools_dir="$(dirname "$0")/../tools"
    
    # Build and run payload generator
    cd "$tools_dir"
    cargo build --release
    
    # Generate small test payload
    log "Generating test payload with 1000 pairs..."
    ./target/release/gen_payload 1000 > "$payload_file"
    
    # Test the payload
    log "Testing generated payload..."
    local start_time=$(date +%s.%N)
    
    if curl -s -X POST \
        -H "Content-Type: application/json" \
        -d "@${payload_file}" \
        "${API_URL}/xml-compare-api/api/compare/xml/batch" \
        -o "$response_file"; then
        
        local end_time=$(date +%s.%N)
        local duration=$(echo "$end_time - $start_time" | bc)
        
        # Parse response
        local total_comparisons=$(jq -r '.total_comparisons // 0' "$response_file")
        local successful_comparisons=$(jq -r '.successful_comparisons // 0' "$response_file")
        
        log "Payload test results:"
        log "  Duration: ${duration}s"
        log "  Total comparisons: $total_comparisons"
        log "  Successful: $successful_comparisons"
        log "  Pairs per second: $(echo "scale=2; $total_comparisons / $duration" | bc)"
        
        success "Payload generator test completed"
    else
        error "Payload test failed"
        return 1
    fi
}

# Function to collect system metrics
collect_system_metrics() {
    log "Collecting system metrics..."
    
    local metrics_file="${TEST_SESSION_DIR}/system_metrics.log"
    
    {
        echo "=== System Information ==="
        uname -a
        echo ""
        
        echo "=== CPU Information ==="
        if command_exists lscpu; then
            lscpu
        elif [ -f /proc/cpuinfo ]; then
            grep "model name\|processor\|cpu cores" /proc/cpuinfo | head -20
        else
            sysctl -n machdep.cpu.brand_string 2>/dev/null || echo "CPU info not available"
        fi
        echo ""
        
        echo "=== Memory Information ==="
        if command_exists free; then
            free -h
        else
            vm_stat 2>/dev/null || echo "Memory info not available"
        fi
        echo ""
        
        echo "=== Disk Usage ==="
        df -h
        echo ""
        
        echo "=== Load Average ==="
        uptime
        echo ""
        
        echo "=== Network Configuration ==="
        if command_exists ss; then
            ss -tuln | grep ":${API_PORT}"
        else
            netstat -tuln 2>/dev/null | grep ":${API_PORT}" || echo "Port ${API_PORT} status unknown"
        fi
        
    } > "$metrics_file"
    
    success "System metrics collected"
}

# Function to generate final report
generate_report() {
    log "Generating performance test report..."
    
    local report_file="${TEST_SESSION_DIR}/performance_report.md"
    
    cat > "$report_file" << EOF
# XML-Compare-API Performance Test Report

**Test Session:** ${TIMESTAMP}  
**API URL:** ${API_URL}  
**Test Date:** $(date)  

## Test Configuration

- Smoke Test Size: ${SMOKE_SIZE} pairs
- Nominal Test Size: ${NOMINAL_SIZE} pairs  
- Stress Test Size: ${STRESS_SIZE} pairs

## Test Results

EOF

    # Add k6 results if available
    if [ -f "${TEST_SESSION_DIR}/k6_summary.json" ]; then
        echo "### k6 Test Results" >> "$report_file"
        echo "" >> "$report_file"
        echo '```json' >> "$report_file"
        cat "${TEST_SESSION_DIR}/k6_summary.json" >> "$report_file"
        echo '```' >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    # Add wrk results if available
    if [ -f "${TEST_SESSION_DIR}/wrk_output.log" ]; then
        echo "### WRK Test Results" >> "$report_file"
        echo "" >> "$report_file"
        echo '```' >> "$report_file"
        tail -20 "${TEST_SESSION_DIR}/wrk_output.log" >> "$report_file"
        echo '```' >> "$report_file"
        echo "" >> "$report_file"
    fi
    
    echo "## System Information" >> "$report_file"
    echo "" >> "$report_file"
    echo '```' >> "$report_file"
    cat "${TEST_SESSION_DIR}/system_metrics.log" >> "$report_file"
    echo '```' >> "$report_file"
    
    success "Performance report generated: $report_file"
}

# Function to cleanup on exit
cleanup() {
    log "Cleaning up..."
    stop_api_server
}

# Main execution
main() {
    log "Starting XML-Compare-API Performance Tests"
    log "Test session directory: $TEST_SESSION_DIR"
    
    # Setup
    mkdir -p "$TEST_SESSION_DIR"
    trap cleanup EXIT
    
    # Collect system info first
    collect_system_metrics
    
    # Start API server
    start_api_server
    
    # Run tests
    run_payload_generator_test
    run_k6_tests
    run_wrk_tests
    
    # Generate report
    generate_report
    
    success "All performance tests completed!"
    log "Results available in: $TEST_SESSION_DIR"
}

# Check dependencies
check_dependencies() {
    local missing_deps=()
    
    if ! command_exists cargo; then
        missing_deps+=("cargo (Rust)")
    fi
    
    if ! command_exists curl; then
        missing_deps+=("curl")
    fi
    
    if ! command_exists jq; then
        missing_deps+=("jq")
    fi
    
    if [ ${#missing_deps[@]} -gt 0 ]; then
        error "Missing required dependencies:"
        printf '%s\n' "${missing_deps[@]}"
        error "Please install missing dependencies and try again"
        exit 1
    fi
}

# Entry point
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    check_dependencies
    main "$@"
fi
