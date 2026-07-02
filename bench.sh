#!/usr/bin/env bash
#
# WebR Framework Performance Benchmark Script (wrk-based)
#
# Prerequisites:
#   - wrk installed (https://github.com/wg/wrk)
#   - A WebR example running, e.g.:
#       cd examples/hello-world && cargo run
#
# Usage:
#   ./bench.sh [base_url] [concurrency] [duration] [threads]
#
# Defaults:
#   base_url   = http://127.0.0.1:8080
#   concurrency= 100
#   duration   = 10s
#   threads    = 4

set -euo pipefail

# ─── Configuration ──────────────────────────────────────────────────

BASE_URL="${1:-http://127.0.0.1:8080}"
CONCURRENCY="${2:-100}"
DURATION="${3:-3s}"
THREADS="${4:-4}"
REPORT_DIR="bench_results"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

# ─── Endpoint Definitions ──────────────────────────────────────────
# Format: "METHOD|PATH|DESCRIPTION"

ENDPOINTS=(
    "GET|/|Simple text response with DI config injection"
    "GET|/health|Health check endpoint (StatusCode only)"
    "GET|/info|Text response with formatted config values"
    "GET|/items|JSON array response (mock data)"
    "GET|/items/1|Path parameter extraction + JSON response"
    "POST|/items|JSON body deserialization + validation + response"
    "PUT|/items/1|JSON body update with path parameter"
    "DELETE|/items/1|Delete with path parameter (204 response)"
)

# Request bodies for endpoints that need them
declare -A POST_BODIES
POST_BODIES["POST|/items"]='{"name":"bench-item"}'
POST_BODIES["PUT|/items/1"]='{"name":"bench-item-updated"}'

# ─── Helper Functions ───────────────────────────────────────────────

SEPARATOR="$(printf '=%.0s' {1..78})"
THIN_SEP="$(printf -- '-%.0s' {1..78})"

check_wrk() {
    if ! command -v wrk &>/dev/null; then
        echo ""
        echo "  ERROR: wrk is not installed."
        echo "  Install it via:"
        echo "    Ubuntu/Debian : sudo apt install wrk"
        echo "    macOS         : brew install wrk"
        echo "    From source   : https://github.com/wg/wrk"
        echo ""
        exit 1
    fi
}

check_server() {
    local status
    status=$(curl -s -o /dev/null -w "%{http_code}" "${BASE_URL}/health" 2>/dev/null || echo "000")
    if [ "$status" = "000" ]; then
        echo ""
        echo "  ERROR: Cannot connect to ${BASE_URL}"
        echo "  Make sure the WebR server is running first."
        echo ""
        exit 1
    fi
    if [ "$status" != "200" ]; then
        echo "  WARNING: /health returned status ${status}"
    fi
}

# Convert latency value to milliseconds
# Input: "756.47us", "1.90ms", "53.18ms", etc.
# Output: "0.76ms", "1.90ms", "53.18ms"
to_ms() {
    local val="$1"
    [ -z "$val" ] && echo "N/A" && return
    local num unit
    num=$(echo "$val" | sed 's/[^0-9.]//g')
    unit=$(echo "$val" | sed 's/[0-9.]//g')
    case "$unit" in
        us) printf "%.2fms" "$(echo "scale=4; $num / 1000" | bc)" ;;
        ms) printf "%.2fms" "$num" ;;
        s)  printf "%.2fms" "$(echo "scale=4; $num * 1000" | bc)" ;;
        *)  echo "${val}" ;;
    esac
}

# Parse wrk output into structured data
# wrk Thread Stats format:
#   Thread Stats   Avg      Stdev     Max   +/- Stdev
#     Latency   756.47us    1.90ms  53.18ms   97.42%
#     Req/Sec    42.63k    10.78k   58.17k    71.75%
parse_wrk_output() {
    local output="$1"

    # Requests/sec
    local rps
    rps=$(echo "$output" | grep "Requests/sec" | awk '{print $2}')

    # Transfer/sec
    local throughput
    throughput=$(echo "$output" | grep "Transfer/sec" | awk '{print $2, $3}')

    # Total requests
    local total_reqs
    total_reqs=$(echo "$output" | grep "requests in" | awk '{print $1}')

    # Latency: Avg, Stdev, Max (from "Latency" line in Thread Stats)
    local lat_avg_raw lat_stdev_raw lat_max_raw
    lat_avg_raw=$(echo "$output" | awk '/Latency/ && /us|ms|s/ && !/Req/ {print $2; exit}')
    lat_stdev_raw=$(echo "$output" | awk '/Latency/ && /us|ms|s/ && !/Req/ {print $3; exit}')
    lat_max_raw=$(echo "$output" | awk '/Latency/ && /us|ms|s/ && !/Req/ {print $4; exit}')
    
    local lat_avg lat_stdev lat_max
    lat_avg=$(to_ms "$lat_avg_raw")
    lat_stdev=$(to_ms "$lat_stdev_raw")
    lat_max=$(to_ms "$lat_max_raw")

    # Errors (connect, read, write, timeout)
    local connect_err read_err write_err timeout_err
    connect_err=$(echo "$output" | awk '/Socket errors.*Connect/ || /Connect/ {for(i=1;i<=NF;i++) if($i=="Connect") print $(i+1)}' | head -1)
    read_err=$(echo "$output" | awk '/Socket errors.*Read/ || /Read/ {for(i=1;i<=NF;i++) if($i=="Read") print $(i+1)}' | head -1)
    write_err=$(echo "$output" | awk '/Socket errors.*Write/ || /Write/ {for(i=1;i<=NF;i++) if($i=="Write") print $(i+1)}' | head -1)
    timeout_err=$(echo "$output" | awk '/Socket errors.*Timeout/ || /Timeout/ {for(i=1;i<=NF;i++) if($i=="Timeout") print $(i+1)}' | head -1)
    [ -z "$connect_err" ] && connect_err=0
    [ -z "$read_err" ] && read_err=0
    [ -z "$write_err" ] && write_err=0
    [ -z "$timeout_err" ] && timeout_err=0

    echo "${rps}|${throughput}|${total_reqs}|${lat_avg}|${lat_stdev}|${lat_max}|${connect_err}|${read_err}|${write_err}|${timeout_err}"
}

# ─── Print Header ──────────────────────────────────────────────────

echo ""
echo "$SEPARATOR"
echo "          WebR Framework Performance Benchmark (wrk)"
echo "$SEPARATOR"
echo "  Target      : ${BASE_URL}"
echo "  Concurrency : ${CONCURRENCY}"
echo "  Duration    : ${DURATION}"
echo "  Threads     : ${THREADS}"
echo "  Endpoints   : ${#ENDPOINTS[@]}"
echo "$SEPARATOR"
echo ""

# ─── Pre-flight Checks ─────────────────────────────────────────────

check_wrk
check_server

# ─── Setup Report Directory ────────────────────────────────────────

mkdir -p "${REPORT_DIR}"

# ─── Run Benchmarks ────────────────────────────────────────────────

declare -a RAW_OUTPUTS
declare -a PARSED_RESULTS

total_endpoints=${#ENDPOINTS[@]}

for i in "${!ENDPOINTS[@]}"; do
    IFS='|' read -r method path desc <<< "${ENDPOINTS[$i]}"
    url="${BASE_URL}${path}"
    idx=$((i + 1))

    printf "  [%d/%d] %-6s %-25s ... " "$idx" "$total_endpoints" "${method}" "${path}"

    # Build wrk command
    wrk_cmd="wrk -t${THREADS} -c${CONCURRENCY} -d${DURATION}"

    # Add request body if needed (POST/PUT)
    if [ "$method" = "POST" ] || [ "$method" = "PUT" ]; then
        local_body="${POST_BODIES["${method}|${path}"]:-}"
        if [ -n "$local_body" ]; then
            lua_script="${REPORT_DIR}/post_${idx}.lua"
            cat > "$lua_script" <<LUA
wrk.method = "${method}"
wrk.body   = '${local_body}'
wrk.headers["Content-Type"] = "application/json"
LUA
            wrk_cmd="${wrk_cmd} -s ${lua_script}"
        fi
    fi

    wrk_cmd="${wrk_cmd} ${url}"

    # Execute wrk
    raw_output=$(eval "$wrk_cmd" 2>&1) || true
    RAW_OUTPUTS+=("$raw_output")

    # Parse results
    parsed=$(parse_wrk_output "$raw_output")
    PARSED_RESULTS+=("$parsed")

    # Print progress
    rps=$(echo "$parsed" | cut -d'|' -f1)
    lat_avg=$(echo "$parsed" | cut -d'|' -f4)
    echo "done  ${rps} req/s  lat_avg=${lat_avg}"
done

echo ""

# ─── Summary Table ─────────────────────────────────────────────────

echo "$SEPARATOR"
echo "  SUMMARY"
echo "$SEPARATOR"
printf "  %-28s %12s %10s %10s %10s %12s %8s\n" \
    "Endpoint" "RPS" "Lat(Avg)" "Lat(Stdev)" "Lat(Max)" "Transfer/s" "Errors"
echo "$THIN_SEP"

total_errors=0

for i in "${!ENDPOINTS[@]}"; do
    IFS='|' read -r method path desc <<< "${ENDPOINTS[$i]}"
    parsed="${PARSED_RESULTS[$i]}"

    IFS='|' read -r rps throughput total_reqs lat_avg lat_stdev lat_max conn_err read_err write_err timeout_err <<< "$parsed"

    errors=$((conn_err + read_err + write_err + timeout_err))
    total_errors=$((total_errors + errors))

    label="${method} ${path}"
    printf "  %-28s %12s %10s %10s %10s %12s %8d\n" \
        "$label" "$rps" "$lat_avg" "$lat_stdev" "$lat_max" "$throughput" "$errors"
done

echo "$THIN_SEP"
echo "  Total Errors: ${total_errors}"
echo "  Timestamp   : $(date '+%Y-%m-%d %H:%M:%S')"
echo "$SEPARATOR"
echo ""

# ─── Save Raw Results ──────────────────────────────────────────────

for i in "${!ENDPOINTS[@]}"; do
    IFS='|' read -r method path desc <<< "${ENDPOINTS[$i]}"
    raw_file="${REPORT_DIR}/${TIMESTAMP}_$(echo "${method}_${path}" | tr '/' '_').txt"
    echo "${RAW_OUTPUTS[$i]}" > "$raw_file"
done

echo "  Raw results saved to: ${REPORT_DIR}/"
echo ""
