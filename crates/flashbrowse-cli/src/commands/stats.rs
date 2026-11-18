//! `flashbrowse stats` - aggregated metrics from the metrics trail.

use flashbrowse_core::observability::{read_records, Stats};

/// Print aggregated statistics for the last `limit` operations.
pub fn run(limit: usize, json_output: bool) {
    let records = read_records(limit);
    let stats = Stats::from(&records);
    if json_output {
        let out = serde_json::json!({
            "window": limit,
            "count": stats.count,
            "failures": stats.failures,
            "error_rate": stats.error_rate,
            "p50_ms": stats.p50_ms,
            "p95_ms": stats.p95_ms,
            "max_ms": stats.max_ms,
            "avg_ms": stats.avg_ms,
            "p50_tokens": stats.p50_tokens,
            "total_tokens": stats.total_tokens,
            "avg_actions": stats.avg_actions,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
        return;
    }
    if stats.count == 0 {
        println!("no metrics recorded yet (metrics live in ~/.cache/flashbrowse/metrics.jsonl)");
        return;
    }
    println!("flashbrowse stats  (last {} operations)", stats.count);
    println!(
        "  latency   p50 {} ms   p95 {} ms   max {} ms   avg {} ms",
        stats.p50_ms, stats.p95_ms, stats.max_ms, stats.avg_ms
    );
    println!(
        "  payload   p50 {} tokens   total {} tokens   avg {} actions",
        stats.p50_tokens, stats.total_tokens, stats.avg_actions
    );
    println!(
        "  errors    {} ({:.1}%)",
        stats.failures,
        stats.error_rate * 100.0
    );
}
