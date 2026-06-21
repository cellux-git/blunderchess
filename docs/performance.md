# Performance

Release build, startpos, 1 thread, warm TT (shared across iterative deepening) unless otherwise noted.

## NPS vs Depth

| Depth | Nodes | Time (ms) | NPS |
|-------|-------|-----------|-----|
| 3 | 4,263 | 2 | 2.13M |
| 4 | 20,483 | 11 | 1.86M |
| 5 | 28,692 | 14 | 2.05M |
| 6 | 127,655 | 71 | 1.80M |
| 7 | 416,349 | 246 | 1.69M |
| 8 | 594,360 | 368 | 1.62M |

Steady ≥1.6M NPS from depth 3+, peaking at 2.13M at depth 3. Node counts at depth 7+ increased after the SEE en-passant fix (2026-06-19) changed quiescence pruning behavior — the fix unblocked previously-incorrectly-pruned en-passant captures in QS, producing a different search tree. Per-node NPS is unchanged; the tree shape differs.

## Lazy SMP Scaling

Release build, startpos, depth 8. TT size scales 8 MB × thread count to prevent thrashing. Fresh TT per run. Total NPS is summed across all threads.

| Threads | TT (MB) | Total nodes | Time (ms) | Total NPS | vs t1 | Efficiency |
|---------|---------|-------------|-----------|-----------|-------|------------|
| 1 | 8 | 958,685 | 687 | 1.40M | 1.00× | 100% |
| 2 | 16 | 1,370,351 | 730 | 1.88M | 1.35× | 67% |
| 4 | 32 | 1,795,753 | 571 | 3.14M | 2.25× | 56% |
| 8 | 64 | 2,381,505 | 351 | 6.78M | 4.86× | 61% |
| 16 | 128 | 2,939,050 | 261 | 11.3M | 8.07× | 50% |

QS TT stores are throttled to Exact/LowerBound entries only, reducing multi-threaded atomic contention. Cold-TT node counts are ~2-3× higher than the warm-TT NPS-vs-depth table above (e.g., depth 8: 959K cold vs 349K warm) — the warm table reflects accumulated entries from prior iterative deepening depths.

### Deep Scaling (16 threads vs 1 thread by search depth)

All runs with fresh TT (8 MB × thread count). Higher node counts than warm-TT single-depth benchmarks.

| Depth | 1T nodes | 1T time | 1T NPS | 16T NPS | Speedup | Efficiency |
|-------|----------|---------|--------|---------|---------|------------|
| 10 | 6,178,045 | 4.7s | 1.32M | 7.94M | 6.03× | 38% |

The 4-way bucket TT with 64-byte-aligned 128-byte padding eliminates most cache-line false sharing between worker threads. TT-in-QS (Exact/LowerBound stores only), IIR, razor pruning, and delta pruning all contribute to per-thread throughput.

## Perft Speed (kiwipete, release)

| Depth | Nodes | Time (ms) | NPS |
|-------|-------|-----------|-----|
| 1 | 48 | <1 | — |
| 2 | 2,039 | <1 | — |
| 3 | 97,862 | 8 | 12.2M |

Perft speed is ~12M NPS (pin recomputation conditional on pin-axis membership — see ADR-0011).
