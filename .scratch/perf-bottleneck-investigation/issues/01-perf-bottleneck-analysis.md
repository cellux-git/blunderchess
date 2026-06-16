# Performance bottleneck analysis

Status: `completed`
Category: `enhancement`

## Summary

Deep investigation of single-thread NPS and Lazy SMP scaling (capped at ~1.37×). Found one critical bottleneck that dominates cost at every search node, plus several high-impact and medium-impact optimizations.

## Verification baseline

All gains must be measured against the numbers in `CONTEXT.md`:

| Benchmark | Metric | Current |
|-----------|--------|---------|
| `bench_nps_vs_depth` (d6-10) | NPS | ≥100K |
| `bench_nps_vs_depth` (d5) | Time | <500ms |
| `bench_thread_scaling` (d8) | 2-thread speedup | ~1.37× vs t1 |
| `bench_thread_scaling` (d8) | 4-thread speedup | ~1.36× vs t1 |
| `bench_perft_speed` (d3) | NPS | ~4.9M |

All 132 tests must continue to pass.

## Critical bottleneck (~20-35% NPS gain)

### 1. `check_result()` called at top of alpha_beta before TT probe

**Location**: `src/search/alpha_beta.rs:32` — before the TT probe at line 40, before null move, before move generation.

**What happens per alpha_beta node**:
1. Calls `generate_legal_moves()` which internally **clones the board** (`src/movegen.rs:10`)
2. Makes/unmakes every pseudo-legal move through the clone (40-100+ make/unmake pairs per node)
3. Then `check_result` does its own `self.clone()` and **another** make/unmake cycle for all legal moves (`src/board.rs:705-713`)
4. Calls `draw::is_draw_by_rule()` which:
   - `is_insufficient_material()`: **heap-allocates a `Vec<(Piece, Color)>`** and loops 64 squares (`src/draw.rs:22-23`)
   - `is_threefold()`: linear scan of hash history
   - `is_fifty_move()`: trivial field check

At 120K NPS, this means ~120K board clones, ~120K Vec allocations, and millions of redundant make/unmake pairs **per second**.

**Fix**: Remove `check_result()` from the alpha_beta hot path entirely.
- Checkmate/stalemate is already detected at the bottom of alpha_beta when `best_move.is_none()` (`src/search/alpha_beta.rs:192-193`)
- Draw detection (50-move, threefold, insufficient material) should be a **cheap inline check** without board cloning or heap allocation
- For threefold: the hash history is already a `Vec<u64>` — scan it without cloning
- For insufficient material: count pieces from the existing bitboards (no loop over 64 squares, no heap allocation)
- Move the draw check **after** the TT probe so TT hits avoid it entirely

**Estimated gain**: 20-35% NPS improvement (eliminates ~120K board clones/sec + ~120K Vec allocations/sec + ~3M redundant make/unmake pairs/sec)

**Verification**: `cargo test --release --test benchmarks -- --ignored --nocapture` shows NPS increase at all benchmark depths. `cargo test --lib` passes. All tactical benchmark tests still find correct moves.

---

## High-impact bottlenecks (~10-20% combined gain)

### 2. Quiescence uses `generate_legal_moves()` which clones board

**Location**: `src/search/quiescence.rs:24`

QS calls `movegen::generate_legal_moves()` which clones the board internally. The alpha-beta search uses the efficient pre-filter approach (`generate_pseudo_legal` + trivial-legality check + inline `is_attacked_by`). QS should do the same.

**Fix**: Use the same pre-filter pattern from alpha_beta in quiescence: generate pseudo-legal, filter to captures/promotions/qs_depth==0 checks, then validate legality with make/unmake only for non-trivial cases.

**Estimated gain**: 10-15% NPS (QS is a large fraction of total nodes at search depths 5+)

**Verification**: Same benchmarks. All tactical tests still pass (scholar's mate, hanging queen, smothered mate).

### 3. `Eval::default()` constructed on every eval call

**Locations**:
- `src/search/alpha_beta.rs:25` — depth limit check
- `src/search/alpha_beta.rs:73` — static eval for futility
- `src/search/quiescence.rs:10` — depth limit check
- `src/search/quiescence.rs:19` — stand-pat
- `src/move_ordering.rs:29,49` — SEE for each capture

`Eval::default()` creates a fresh struct each time. While the struct fields are all stack data, the call pattern is avoidable. Additionally, `evaluate()` and `game_phase()` each iterate `board.piece_list()` separately — 3 full passes per evaluate call.

**Fix**:
- Make one `static EVAL: Eval` instance and reuse it everywhere
- Merge `game_phase()` into `evaluate_side()` to avoid the extra piece_list pass
- Or pass `&Eval` down from `search_worker` to avoid repeated default construction

**Estimated gain**: 5-10% NPS

**Verification**: Benchmarks showing reduced time at all depths.

---

## Medium-impact bottlenecks (~5-10% combined gain)

### 4. Move ordering `sort_by_cached_key` heap allocation

**Location**: `src/move_ordering.rs:25`

For typical move lists (30-40 moves), `sort_by_cached_key` allocates a Vec of keys on the heap. For small N, an insertion sort or `sort_unstable_by_key` (which allocates via a temporary buffer but may optimize for inline buffers) would be faster.

**Estimated gain**: 3-5% NPS

### 5. `is_insufficient_material` heap allocates Vec

**Location**: `src/draw.rs:22`

Allocates `Vec<(Piece, Color)>` every call. Can be rewritten to process bitboards directly with no allocation. With bottleneck #1 eliminated, this becomes less critical but still worth fixing.

**Estimated gain**: 1-3% NPS (mostly absorbed into bottleneck #1 fix)

### 6. `piece_list` iteration overhead in eval

**Location**: `src/eval/mod.rs:77`

`evaluate_side()` iterates piece_list for material+PST, then each term function separately fetches bitboards and iterates again. The piece_list Vec iteration is less efficient than bitboard population counts and loops. After fixing bottleneck #1, this becomes the dominant cost in eval.

**Estimated gain**: 3-5% NPS

---

## Multi-thread scaling: why 1.37× cap

### 7. TT cache-line contention from all threads writing shared TT

All N threads store to the same lock-free TT using `Ordering::Release` stores. Each TT bucket spans 3× `AtomicU64` = 24 bytes (2-3 cache lines). When multiple cores write to overlapping or nearby buckets, cache coherence protocol bounces the cache lines. This is the primary reason 2 and 4 threads show the same speedup.

**Fix**: 
- Pack TT bucket into 2× u64 (16 bytes, single cache line): use fewer bits for score/depth/age
- Alternatively: per-thread TT regions by masking thread_id into the hash index
- Alternatively: store less aggressively (skip more writes for low-depth entries)

**Estimated gain**: 2-thread speedup from 1.37× to 1.6-1.8×; 4-thread speedup to 2.0-2.5×

**Verification**: `bench_thread_scaling` benchmark shows improved scaling ratios.

### 8. Weak Lazy SMP diversity

**Location**: `src/move_ordering.rs:37-39`

Root perturbation is `thread_id % 16` at ply==0 only. Threads 2+ explore nearly identical trees. Stronger perturbation (larger history bonus offsets, depth-dependent perturbation) would make threads explore more diverse subtrees, improving combined search quality.

**Estimated gain**: Modest NPS impact but could improve effective search depth at equal time.

### 9. Helper threads never stopped independently

**Location**: `src/search/mt.rs:63`

Threads 1..N get `AtomicBool::new(false)` — never set. They run to completion even if thread 0 already finished. With fixed-depth benchmarks this is not a factor, but in time-limited play it wastes CPU.

**Fix**: Share a single stop flag across all threads, or abort helpers when thread 0 completes at the current depth.

### 10. Board clone per thread

**Location**: `src/search/mt.rs:61`

Each thread clones the board (including Vec history). Consider a cheaper copy mechanism (e.g., Arena-based allocation or shallow copy).

**Estimated gain**: Minor (one-time cost per thread spawn)

---

## Implementation plan (vertical slices)

### Slice A (AFK): Remove `check_result()` from alpha_beta hot path
- Move draw detection to a cheap inline function operating directly on bitboards + hash history
- Call it after TT probe (not before)
- Remove internal board clone from `check_result` and `generate_legal_moves` flow
- **Blocked by**: none
- **Estimated gain**: 20-35% NPS

### Slice B (AFK): Replace QS `generate_legal_moves` with pre-filter approach
- Reuse the trivial-legality logic from alpha_beta in quiescence
- **Blocked by**: none (independent of A, but may conflict in diff)
- **Estimated gain**: 10-15% NPS

### Slice C (AFK): Cache Eval default + merge game_phase into evaluate_side
- Make Eval a static/const or thread-local
- Fold phase calculation into the material+PST loop
- **Blocked by**: none
- **Estimated gain**: 5-10% NPS

### Slice D (AFK): Optimize TT for cache-line friendliness
- Pack bucket into 2× u64 (16 bytes)
- **Blocked by**: none
- **Estimated gain**: 2-5% single-thread, bigger multi-thread impact

### Slice E (AFK): Stronger Lazy SMP root perturbation
- Increase perturbation depth beyond ply==0, use larger values
- **Blocked by**: none
- **Estimated gain**: Improved scaling quality, modest NPS

### Slice F (AFK): Remove heap allocations from draw detection
- Rewrite `is_insufficient_material` to use bitboards directly
- **Blocked by**: depends on Slice A (may be subsumed)
- **Estimated gain**: 1-3% NPS (absorbed into A)

---

## Expected cumulative impact

| Phase | NPS change | Thread scaling |
|-------|-----------|----------------|
| Baseline (CONTEXT.md) | 112-133K (d6-10) | 1.37× (2t), 1.36× (4t) |
| After A+B | ~150-180K | same |
| After A+B+C | ~165-200K | same |
| After A+B+C+D | ~175-210K | 1.5-1.7× (2t), 1.8-2.2× (4t) |
| After all | ~200K+ | 1.6-1.8× (2t), 2.0-2.5× (4t) |

---

## Comments

> *This was generated by AI during triage.*

### Agent brief — Slice A: Remove `check_result()` from alpha_beta hot path

**Goal**: Eliminate `board.check_result()` call at `src/search/alpha_beta.rs:32`, which currently runs before the TT probe and clones the board + allocates memory at every visited node.

**What to build**:

Replace the up-front `check_result()` termination check with two lightweight checks that operate directly on existing board state:

1. **Draw detection** — a free function `fn is_terminal_draw(board: &Board) -> bool` that does zero allocations:
   - 50-move: `board.halfmove_clock() >= 100`
   - Threefold: scan `board.history()` for 2+ occurrences of `board.hash()` (Vec already exists, no clone)
   - Insufficient material: use bitboards — count pieces via `board.pieces_bb(Piece::*)` and `board.colors_bb(Color::*)`, apply the same K/K+B/K+N/K+BvB same-color logic. No Vec allocation, no 64-square loop.

2. **Call it after the TT probe** — insert the draw check at `src/search/alpha_beta.rs:56`, right after the TT cutoff block (after node type match arms, before the null-move check). If the draw check returns true, return 0 immediately.

3. **Keep the bottom fallthrough** — the checkmate/stalemate detection at `src/search/alpha_beta.rs:192-193` (`if best_move.is_none()`) already handles checkmates and stalemates correctly. No change needed there.

4. **Leave `check_result()` in place** — it's still needed for `Board::check_result()` callers outside the search (UCI, tests). Just stop calling it from alpha_beta.

**Files to touch**:
- `src/draw.rs` — add `pub fn is_terminal_draw(board: &Board) -> bool` (allocation-free)
- `src/search/alpha_beta.rs` — remove line 32-37 (`if let Some(result) = board.check_result()`), add `if crate::draw::is_terminal_draw(board) { return 0; }` after the TT cutoff block (after line 55)
- Update imports accordingly

**Don't touch**: `src/board.rs:700` (`check_result`) — keep it for UCI/tests

**Acceptance criteria**:
- [ ] `cargo test --lib` — all 132 tests pass
- [ ] `cargo test --test benchmarks` — all 10 tactical tests pass (scholar's mate, back-rank mate, hanging queen, smothered mate, etc.)
- [ ] `cargo test --release --test benchmarks -- --ignored --nocapture` — `bench_nps_vs_depth` shows NPS increase at all depths vs CONTEXT.md baseline (≥100K at d6+, d5 <500ms)
- [ ] No functional change in best moves found (tactical tests still find correct mates and captures)
- [ ] `bench_perft_speed` still 4.9M NPS at d3
