---
name: optimize-blunderchess-performance
description: Optimize the BlunderChess chess engine. Use when the user asks to optimize the engine, asks to make the engine faster, or wants to improve engine performance.
---

# Optimize BlunderChess

## Workflow

### 1. Gate

Ask the user: "Start the optimization process? I'll read the performance ADR, sweep for improvements, and present a table for you to pick from."

Do not proceed until the user confirms.

**Completion criterion**: User confirmed.

### 2. Pre-flight

Read `docs/adr/0011-performance-optimization.md`. Note:
- 47 active techniques already applied
- 9 rejected attempts with measurements — do not retry unless surrounding code has materially changed
- 3 remaining candidates not yet attempted

**Completion criterion**: Summarized the rejected approaches and 3 remaining candidates to the user.

### 3. Sweep

Inspect hot paths for opportunities beyond what ADR-0011 lists. Hot paths: `alpha_beta`, `quiescence`, `evaluate`, `movegen`, `make_move`/`unmake_move`, TT `probe`/`store`, move ordering, `is_attacked_by`, `compute_pinned_impl`.

Cross-check every candidate against ADR-0011's rejected list. If proposing something already tried and rejected, explain what materially changed.

**Completion criterion**: A list of candidates, each tied to a specific code location with rationale.

### 4. Propose

Present a table:

| # | Description | Est. Impact | Change Size | Regression Risk |
|---|-------------|-------------|-------------|-----------------|

- **Est. Impact**: Low / Medium / High NPS lift
- **Change Size**: Small / Medium / Large
- **Regression Risk**: Low / Medium / High

Include ADR-0011's remaining candidates and sweep findings. Flag High-risk candidates — the user may exclude them.

**Completion criterion**: Table presented, user responded with selections.

### 5. Baseline

Run the release benchmarks 5 times and average the NPS:

```bash
for i in 1 2 3 4 5; do
    echo "=== Run $i ==="
    cargo test --release --test benchmarks -- --ignored --nocapture
done
```

Record the average NPS at each depth and per thread count.

**Completion criterion**: Average baseline NPS from 5 runs recorded.

### 6. Optimize

For each selected candidate, in order:

```
IMPLEMENT → cargo check → cargo test --lib --test benchmarks → release benchmarks → DECIDE
```

Rules:
- One change per iteration
- `cargo test --lib --test benchmarks` after each change
- Release benchmarks after each change: run 5 times and average NPS
- **Tangible NPS gain**: keep
- **No measurable gain**: keep only if zero complexity increase. Revert otherwise
- **Test breakage or NPS regression**: revert immediately. Append a brief note to ADR-0011 under "Approaches that did not work out" — what was tried, what broke, the numbers

**Completion criterion**: Every selected candidate attempted, every outcome recorded.

### 7. Close

Print final NPS delta against baseline. Confirm ADR-0011 is updated for any reverted attempts.

**Completion criterion**: Final NPS vs baseline printed, ADR up to date.
