#!/usr/bin/env bash
set -euo pipefail

# ─── Automated benchmark matrix runner ─────────────────────────────────
# Reads scenarios from a TSV manifest, runs each for both Rust and C++
# backends, aggregates results into a structured output directory.
#
# Usage:
#   scripts/xlog/run_bench_matrix.sh --manifest <tsv> --out-root <dir> [options]
#
# Options:
#   --manifest <file>   Path to TSV manifest (required)
#   --out-root <dir>    Output root directory (required)
#   --runs <n>          Runs per scenario per backend (default: 3)
#   --backends <list>   Comma-separated backends: rust,cpp (default: rust,cpp)
#   --filter <pattern>  Only run scenarios matching this grep pattern
#   --skip-build        Skip cargo build step
#   --components        Also run component micro-benchmarks
#   -h, --help          Show help

manifest=""
out_root=""
runs=3
backends="rust,cpp"
filter=""
skip_build=0
run_components=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --manifest)     manifest="$2";     shift 2 ;;
    --out-root)     out_root="$2";     shift 2 ;;
    --runs)         runs="$2";         shift 2 ;;
    --backends)     backends="$2";     shift 2 ;;
    --filter)       filter="$2";       shift 2 ;;
    --skip-build)   skip_build=1;      shift   ;;
    --components)   run_components=1;  shift   ;;
    -h|--help)
      sed -n '3,/^$/p' "$0" | sed 's/^# \?//'
      exit 0
      ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done

if [[ -z "$manifest" ]]; then
  echo "error: --manifest is required" >&2
  exit 2
fi
if [[ -z "$out_root" ]]; then
  echo "error: --out-root is required" >&2
  exit 2
fi
if [[ ! -f "$manifest" ]]; then
  echo "error: manifest file not found: $manifest" >&2
  exit 2
fi

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/../.." && pwd)"

mkdir -p "$out_root"
cp "$manifest" "$out_root/manifest.tsv"

results_raw="$out_root/results_raw.jsonl"
: > "$results_raw"
log_file="$out_root/run.log"
: > "$log_file"
started_at_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
started_epoch="$(date +%s)"

log() {
  echo "[$(date +%H:%M:%S)] $*" | tee -a "$log_file"
}

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

detect_cpu_count() {
  if command -v getconf >/dev/null 2>&1; then
    getconf _NPROCESSORS_ONLN 2>/dev/null && return 0
  fi
  if command -v sysctl >/dev/null 2>&1; then
    sysctl -n hw.ncpu 2>/dev/null && return 0
  fi
  echo "unknown"
}

reverse_backends() {
  local arr=("$@")
  local idx
  for ((idx=${#arr[@]} - 1; idx >= 0; idx--)); do
    printf '%s\n' "${arr[idx]}"
  done
}

backends_json_array() {
  local arr=("$@")
  local first=1
  printf '['
  for be in "${arr[@]}"; do
    [[ -z "$be" ]] && continue
    if [[ "$first" -eq 0 ]]; then
      printf ','
    fi
    printf '"%s"' "$(json_escape "$be")"
    first=0
  done
  printf ']'
}

IFS=',' read -ra requested_backends <<< "$backends"
for i in "${!requested_backends[@]}"; do
  requested_backends[$i]="${requested_backends[$i]// /}"
done

# ─── Build step ─────────────────────────────────────────────────────────
if [[ "$skip_build" -eq 0 ]]; then
  for be in "${requested_backends[@]}"; do
    log "Building ${be}-backend (release)..."
    cargo build --release -p mars-xlog --example bench_backend \
      --no-default-features --features "${be}-backend" 2>&1 | tail -1 | tee -a "$log_file"
  done
fi

# ─── Component micro-benchmarks ────────────────────────────────────────
if [[ "$run_components" -eq 1 ]]; then
  log "Running component micro-benchmarks..."
  comp_out="$out_root/components.jsonl"
  : > "$comp_out"

  for size in 16 96 256 1024 4096; do
    cargo run --release -p mars-xlog-core --example bench_components -- \
      all --iterations 100000 --payload-size "$size" 2>>"$log_file" | tee -a "$comp_out"
  done
  log "Component benchmarks complete → $comp_out"
fi

# ─── Read manifest and run scenarios ────────────────────────────────────
scenario_count=0
while IFS=$'\t' read -r scenario messages mode threads compress compress_level msg_size flush_every cache_days max_file_size pub_key warmup time_buckets; do
  # Skip header and comments
  [[ "$scenario" =~ ^#.*$ ]] && continue
  [[ "$scenario" == "scenario" ]] && continue
  [[ -z "$scenario" ]] && continue

  # Apply filter
  if [[ -n "$filter" ]] && ! echo "$scenario" | grep -qE "$filter"; then
    continue
  fi

  scenario_count=$((scenario_count + 1))
  log "━━━ Scenario: $scenario (messages=$messages, mode=$mode, threads=$threads, compress=$compress, lv=$compress_level, msg_size=$msg_size) ━━━"

  mkdir -p "$out_root/${scenario}"
  for be in "${requested_backends[@]}"; do
    : > "$out_root/${scenario}/results_${be}.jsonl"
  done

  for run_idx in $(seq 1 "$runs"); do
    backend_order=("${requested_backends[@]}")
    if (( ${#backend_order[@]} > 1 )) && (( (scenario_count + run_idx) % 2 == 0 )); then
      backend_order=()
      while IFS= read -r reversed_backend; do
        backend_order+=("$reversed_backend")
      done < <(reverse_backends "${requested_backends[@]}")
    fi
    log "  run ${run_idx}/${runs} backend order: ${backend_order[*]}"

    for be in "${backend_order[@]}"; do
      feature="${be}-backend"
      results_file="$out_root/${scenario}/results_${be}.jsonl"
      run_dir="$out_root/${scenario}/${be}-run${run_idx}"
      cache_dir_arg=""
      rm -rf "$run_dir"

      cmd=(
        cargo run --release -p mars-xlog --example bench_backend
        --no-default-features --features "$feature" --
        --out-dir "$run_dir"
        --prefix "${scenario}-${be}"
        --messages "$messages"
        --mode "$mode"
        --compress "$compress"
        --compress-level "$compress_level"
        --msg-size "$msg_size"
        --threads "$threads"
        --flush-every "$flush_every"
        --warmup "${warmup:-500}"
        --max-file-size "$max_file_size"
      )

      if [[ -n "${time_buckets:-}" ]] && [[ "$time_buckets" -gt 0 ]]; then
        cmd+=(--time-buckets "$time_buckets")
      fi

      if [[ -n "${pub_key:-}" ]] && [[ "$pub_key" != "" ]]; then
        cmd+=(--pub-key "$pub_key")
      fi

      if [[ "${cache_days:-0}" -gt 0 ]]; then
        cache_dir_arg="$out_root/${scenario}/${be}-cache${run_idx}"
        rm -rf "$cache_dir_arg"
        cmd+=(--cache-dir "$cache_dir_arg" --cache-days "$cache_days")
      fi

      log "  [${be}] run ${run_idx}/${runs}..."
      if output=$("${cmd[@]}" 2>>"$log_file"); then
        echo "$output" >> "$results_file"
        printf '{"scenario":"%s","backend":"%s","run_index":%d,"run_dir":"%s","result":%s}\n' \
          "$(json_escape "$scenario")" \
          "$(json_escape "$be")" \
          "$run_idx" \
          "$(json_escape "$run_dir")" \
          "$output" >> "$results_raw"
        # Extract throughput for quick display
        tps=$(echo "$output" | grep -o '"throughput_mps":[0-9.]*' | cut -d: -f2)
        log "  [${be}] run ${run_idx}: ${tps:-?} mps"
      else
        log "  [${be}] run ${run_idx}: FAILED (exit=$?)"
      fi
    done
  done
done < "$manifest"

log "━━━ Matrix complete: ${scenario_count} scenarios × ${runs} runs ━━━"
log "Raw results: $results_raw"
log "Log: $log_file"

# ─── Generate summary ──────────────────────────────────────────────────
summary_file="$out_root/summary.md"
summary_json="$out_root/summary.json"
summary_rows_tmp="$out_root/.summary_rows.tsv"
: > "$summary_rows_tmp"

for scenario_dir in "$out_root"/*/; do
  scenario_name=$(basename "$scenario_dir")
  for results_file in "$scenario_dir"/results_*.jsonl; do
    [[ -f "$results_file" ]] || continue
    be=$(basename "$results_file" | sed 's/results_//;s/\.jsonl//')

    if [[ -s "$results_file" ]]; then
      row=$(
        awk -F'[,:]' '
        BEGIN { n=0; tps=0; avg=0; p99=0; p999=0; bpm=0 }
        {
          for(i=1;i<=NF;i++) {
            gsub(/["{} ]/, "", $i)
            if($i=="throughput_mps") { tps+=$(i+1); }
            if($i=="lat_avg_ns") { avg+=$(i+1); }
            if($i=="lat_p99_ns") { p99+=$(i+1); }
            if($i=="lat_p999_ns") { p999+=$(i+1); }
            if($i=="bytes_per_msg") { bpm+=$(i+1); }
          }
          n++
        }
        END {
          if(n>0) printf "%s\t%s\t%.3f\t%.3f\t%.3f\t%.3f\t%.3f\n",
            scenario, be, tps/n, avg/n, p99/n, p999/n, bpm/n
        }
        ' scenario="$scenario_name" be="$be" "$results_file"
      )
      if [[ -n "$row" ]]; then
        printf '%s\n' "$row" >> "$summary_rows_tmp"
      fi
    fi
  done
done

{
  echo "# Benchmark Matrix Summary"
  echo ""
  echo "| Scenario | Backend | Throughput (mps) | Avg Lat (ns) | P99 Lat (ns) | P999 Lat (ns) | Output (bytes/msg) |"
  echo "| :--- | :--- | ---: | ---: | ---: | ---: | ---: |"
  while IFS=$'\t' read -r scenario_name be tps avg p99 p999 bpm; do
    printf '| %s | %s | %.0f | %.0f | %.0f | %.0f | %.1f |\n' \
      "$scenario_name" "$be" "$tps" "$avg" "$p99" "$p999" "$bpm"
  done < "$summary_rows_tmp"
} > "$summary_file"

{
  echo "["
  first=1
  while IFS=$'\t' read -r scenario_name be tps avg p99 p999 bpm; do
    if [[ "$first" -eq 0 ]]; then
      echo ","
    fi
    printf '  {"scenario":"%s","backend":"%s","throughput_mps":%s,"lat_avg_ns":%s,"lat_p99_ns":%s,"lat_p999_ns":%s,"bytes_per_msg":%s}' \
      "$(json_escape "$scenario_name")" \
      "$(json_escape "$be")" \
      "$tps" \
      "$avg" \
      "$p99" \
      "$p999" \
      "$bpm"
    first=0
  done < "$summary_rows_tmp"
  echo
  echo "]"
} > "$summary_json"

finished_at_utc="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
finished_epoch="$(date +%s)"
metadata_file="$out_root/metadata.json"
{
  echo "{"
  printf '  "started_at_utc":"%s",\n' "$started_at_utc"
  printf '  "finished_at_utc":"%s",\n' "$finished_at_utc"
  printf '  "duration_seconds":%d,\n' "$((finished_epoch - started_epoch))"
  printf '  "hostname":"%s",\n' "$(json_escape "$(hostname)")"
  printf '  "os":"%s",\n' "$(json_escape "$(uname -s)")"
  printf '  "arch":"%s",\n' "$(json_escape "$(uname -m)")"
  printf '  "cpu_count":"%s",\n' "$(json_escape "$(detect_cpu_count)")"
  printf '  "git_branch":"%s",\n' "$(json_escape "$(git -C "$repo_root" branch --show-current 2>/dev/null || echo unknown)")"
  printf '  "git_commit":"%s",\n' "$(json_escape "$(git -C "$repo_root" rev-parse HEAD 2>/dev/null || echo unknown)")"
  printf '  "cargo_profile":"release",\n'
  printf '  "manifest":"%s",\n' "$(json_escape "$manifest")"
  printf '  "manifest_copy":"%s",\n' "$(json_escape "$out_root/manifest.tsv")"
  printf '  "backends":%s,\n' "$(backends_json_array "${requested_backends[@]}")"
  printf '  "backend_order_policy":"alternating-by-scenario-and-run",\n'
  printf '  "runs":%d,\n' "$runs"
  printf '  "filter":"%s",\n' "$(json_escape "$filter")"
  printf '  "components":%s,\n' "$([[ "$run_components" -eq 1 ]] && echo true || echo false)"
  printf '  "scenario_count":%d,\n' "$scenario_count"
  printf '  "results_raw":"%s",\n' "$(json_escape "$results_raw")"
  printf '  "summary_markdown":"%s",\n' "$(json_escape "$summary_file")"
  printf '  "summary_json":"%s",\n' "$(json_escape "$summary_json")"
  printf '  "run_log":"%s"\n' "$(json_escape "$log_file")"
  echo "}"
} > "$metadata_file"

rm -f "$summary_rows_tmp"

log "Summary: $summary_file"
log "Summary JSON: $summary_json"
log "Metadata: $metadata_file"
