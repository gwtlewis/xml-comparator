#!/bin/bash

# Direct 600k XML performance test
# This script generates and tests a 600k XML payload directly

set -euo pipefail

# Configuration
API_PORT=${APP_PORT:-3000}
API_URL="http://127.0.0.1:${API_PORT}"
TEST_SIZE=600000

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m'

log() {
    echo -e "${BLUE}[$(date +'%H:%M:%S')]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

warn() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Create results directory
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RESULTS_DIR="$(cd "$SCRIPT_DIR/.." && pwd)/results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")
TEST_SESSION_DIR="${RESULTS_DIR}/600k_test_${TIMESTAMP}"
mkdir -p "$TEST_SESSION_DIR"

log "Starting 600k XML Performance Test"
log "Test session directory: $TEST_SESSION_DIR"

# Check if API is running
if ! curl -s "${API_URL}/xml-compare-api/health" >/dev/null 2>&1; then
    error "API is not running at ${API_URL}"
    error "Please start the API first: cargo run --release"
    exit 1
fi

success "API is running at ${API_URL}"

# Generate 600k payload
log "Generating ${TEST_SIZE} XML pairs..."
cd "${SCRIPT_DIR}/../tools"
if ! cargo build --release --quiet; then
    error "Failed to build payload generator"
    exit 1
fi

PAYLOAD_FILE="${TEST_SESSION_DIR}/payload_600k.json"
log "This may take a few minutes..."

if ! ./target/release/gen_payload ${TEST_SIZE} > "$PAYLOAD_FILE"; then
    error "Failed to generate payload"
    exit 1
fi

PAYLOAD_SIZE=$(du -h "$PAYLOAD_FILE" | cut -f1)
success "Generated ${TEST_SIZE} XML pairs (${PAYLOAD_SIZE})"

# Test the payload
log "Testing 600k XML comparison..."
RESPONSE_FILE="${TEST_SESSION_DIR}/response_600k.json"

# Record system state before test
log "Recording system metrics..."
{
    echo "=== Pre-test System State ==="
    echo "Memory usage:"
    if command -v free >/dev/null 2>&1; then
        free -h
    else
        vm_stat
    fi
    echo ""
    echo "Load average:"
    uptime
    echo ""
    echo "Disk space:"
    df -h
} > "${TEST_SESSION_DIR}/system_metrics_before.log"

# Run the test
START_TIME=$(date +%s.%N)
log "Starting test at $(date)"

if curl -X POST \
    -H "Content-Type: application/json" \
    -d "@${PAYLOAD_FILE}" \
    "${API_URL}/xml-compare-api/api/compare/xml/batch" \
    -o "$RESPONSE_FILE" \
    --max-time 3600 \
    --connect-timeout 30; then
    
    END_TIME=$(date +%s.%N)
    DURATION=$(echo "$END_TIME - $START_TIME" | bc)
    
    # Parse response
    if command -v jq >/dev/null 2>&1; then
        TOTAL_COMPARISONS=$(jq -r '.total_comparisons // 0' "$RESPONSE_FILE")
        SUCCESSFUL_COMPARISONS=$(jq -r '.successful_comparisons // 0' "$RESPONSE_FILE")
        FAILED_COMPARISONS=$(jq -r '.failed_comparisons // 0' "$RESPONSE_FILE")
    else
        TOTAL_COMPARISONS="unknown"
        SUCCESSFUL_COMPARISONS="unknown" 
        FAILED_COMPARISONS="unknown"
    fi
    
    # Calculate metrics
    PAIRS_PER_SECOND=$(echo "scale=2; ${TEST_SIZE} / ${DURATION}" | bc)
    RESPONSE_SIZE=$(du -h "$RESPONSE_FILE" | cut -f1)
    
    # Record system state after test
    {
        echo "=== Post-test System State ==="
        echo "Memory usage:"
        if command -v free >/dev/null 2>&1; then
            free -h
        else
            vm_stat
        fi
        echo ""
        echo "Load average:"
        uptime
    } > "${TEST_SESSION_DIR}/system_metrics_after.log"
    
    # Generate report
    {
        echo "# 600k XML Performance Test Report"
        echo ""
        echo "**Test Date:** $(date)"
        echo "**Test Size:** ${TEST_SIZE} XML pairs"
        echo "**API URL:** ${API_URL}"
        echo ""
        echo "## Results"
        echo ""
        echo "- **Duration:** ${DURATION}s ($(echo "scale=2; ${DURATION} / 60" | bc) minutes)"
        echo "- **Throughput:** ${PAIRS_PER_SECOND} pairs/second"
        echo "- **Total Comparisons:** ${TOTAL_COMPARISONS}"
        echo "- **Successful:** ${SUCCESSFUL_COMPARISONS}"
        echo "- **Failed:** ${FAILED_COMPARISONS}"
        echo "- **Payload Size:** ${PAYLOAD_SIZE}"
        echo "- **Response Size:** ${RESPONSE_SIZE}"
        echo ""
        echo "## Performance Analysis"
        echo ""
        if (( $(echo "${DURATION} < 1800" | bc -l) )); then
            echo "✅ **PASS** - Completed within 30 minutes"
        else
            echo "❌ **SLOW** - Took longer than 30 minutes"
        fi
        
        if (( $(echo "${PAIRS_PER_SECOND} > 333" | bc -l) )); then
            echo "✅ **PASS** - Throughput above target (333 pairs/sec)"
        else
            echo "❌ **SLOW** - Throughput below target"
        fi
        
        if [ "$TOTAL_COMPARISONS" = "$TEST_SIZE" ]; then
            echo "✅ **PASS** - All comparisons processed"
        else
            echo "❌ **FAIL** - Some comparisons missing"
        fi
        
        if [ "$FAILED_COMPARISONS" = "0" ]; then
            echo "✅ **PASS** - No failed comparisons"
        else
            echo "❌ **FAIL** - ${FAILED_COMPARISONS} comparisons failed"
        fi
        
    } > "${TEST_SESSION_DIR}/test_report.md"
    
    success "600k XML test completed!"
    log "Duration: ${DURATION}s ($(echo "scale=2; ${DURATION} / 60" | bc) minutes)"
    log "Throughput: ${PAIRS_PER_SECOND} pairs/second"
    log "Results saved to: ${TEST_SESSION_DIR}"
    
    # Display summary
    cat "${TEST_SESSION_DIR}/test_report.md"
    
else
    error "Test failed or timed out"
    log "Check ${TEST_SESSION_DIR}/response_600k.json for details"
    exit 1
fi
