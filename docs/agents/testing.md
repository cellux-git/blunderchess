# Testing

## Unit tests (78, always run)

```bash
cargo test --lib
```

Runs all inline `#[cfg(test)]` tests across `src/*.rs`. Fast — ~3.5s.

## Integration tests (10 tactical + 3 benchmarks)

```bash
cargo test --test benchmarks           # tactical only, ~5s
cargo test --lib --test benchmarks     # both suites, ~8s
```

Tactical tests run in debug mode. Benchmarks are `#[ignore]` d — they only run in `--release`.

## Performance benchmarks (release only)

```bash
cargo test --release --test benchmarks -- --ignored --nocapture
```

~4s total. Benchmarks assert NPS thresholds (≥100K at depth 6+) in release mode only. Debug mode prints `[SKIPPED in debug — use --release]` and returns immediately.

| Benchmark | What it measures |
|-----------|-----------------|
| `bench_nps_vs_depth` | Search depth 3-10 from startpos, 1 thread, shared TT — prints nodes/ms/NPS per depth |
| `bench_thread_scaling` | Depth 8 with 1/2/4 threads, fresh TT each — prints nodes/ms/NPS per thread count |
| `bench_perft_speed` | Kiwipete perft depth 1-3 — prints nodes/ms/NPS |

## Run everything

```bash
cargo test --lib --test benchmarks                          # all tests, ~8s
cargo test --release --test benchmarks -- --ignored --nocapture  # + bench, +4s
```
