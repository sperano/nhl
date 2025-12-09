Run and analyze performance benchmarks.

**Step 1: Run benchmarks**

```bash
cargo bench 2>&1 | tee /tmp/bench_output.txt
```

**Step 2: Parse results**

Extract timing information:
- Function name
- Time per iteration (mean)
- Throughput (if applicable)
- Comparison to baseline (if available)

**Step 3: Identify bottlenecks**

Flag any benchmarks where:
- Mean time > 1ms for rendering operations
- Mean time > 100ms for API operations
- Significant regression from previous run

**Step 4: Profile slow operations**

For flagged benchmarks:
```bash
# Run with flamegraph (if installed)
cargo flamegraph --bench performance -- --bench {bench_name}
```

**Step 5: Generate report**

```
Performance Benchmark Report
============================

Rendering Benchmarks:
---------------------
| Benchmark           | Mean      | Throughput    | Status |
|---------------------|-----------|---------------|--------|
| render_scores_tab   | 0.5ms     | 2000/sec      | OK     |
| render_standings    | 0.3ms     | 3333/sec      | OK     |
| render_full_app     | 1.2ms     | 833/sec       | SLOW   |

Data Benchmarks:
----------------
| Benchmark           | Mean      | Status |
|---------------------|-----------|--------|
| parse_schedule      | 5ms       | OK     |
| parallel_fetch      | 170ms     | OK     |

Recommendations:
----------------
1. {Specific optimization suggestion}
2. {Another suggestion}

Previous Run Comparison:
------------------------
- render_full_app: +15% slower (was 1.0ms)
- parallel_fetch: -10% faster (was 190ms)
```

**Step 6: Quick optimization check**

If slow benchmarks found, check for:
- [ ] Unnecessary allocations in hot paths
- [ ] String formatting in loops
- [ ] Missing `Arc` for shared data
- [ ] Sequential where parallel would help
- [ ] Redundant clones

**Optional: Run specific benchmark**
```bash
cargo bench -- {bench_name}
```

**Optional: Compare to baseline**
```bash
# Save current as baseline
cp target/criterion target/criterion-baseline -r

# Run again and compare
cargo bench -- --baseline baseline
```
