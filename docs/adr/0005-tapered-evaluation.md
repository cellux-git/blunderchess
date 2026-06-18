# Tapered evaluation with tunable parameter struct and SEE

**Status**: accepted

## Context

A chess engine's evaluation function must assess positions quickly and accurately. The design space ranges from simple material counting to neural network inference. For a hobby engine, the goal is a handcrafted evaluation that is strong enough for competent play, fast enough to search deeply, and structured for future tuning.

## Decision

Use a **tapered evaluation** with **PeSTO-derived piece-square tables**, housed in a single **`Eval` struct** with ~40 tunable fields. Positional terms include pawn structure, king safety, mobility, and piece-specific bonuses. The struct is behind a **`OnceLock<Eval>` static default** for zero per-call allocation. **Static Exchange Evaluation (SEE)** lives in the same module.

### Evaluation components

| Category | Midgame term | Endgame term | Details |
|----------|-------------|-------------|---------|
| Material + PST | Per-piece value + PST lookup | Same, different PST | PeSTO-derived, 6 piece types × 64 squares × 2 phases |
| Pawn structure | Doubled (−12), isolated (−12), backward (−8), passed (+0..+120 by rank) | Doubled (−24), isolated (−20), backward (−16), same passed bonuses | Connected passers get +20 bonus |
| King safety | Shield (2-3 squares in front, −45 per missing pawn), open adjacent files (−30), attacker zone (+15 per attacker within 8-zone) | Endgame: king activity encouraged via PST | Zone: Chebyshev distance ≤ 2 from king |
| Mobility | N: weighted count (cap 8), B: (cap 13), R: (cap 14), Q: (cap 27) | Same | Safe squares only (not attacked by enemy pawns) |
| Bishop pair | +40 | +60 | Both bishops present |
| Trapped bishop | Penalty for a2/h2/a7/h7 bishop blocked by own pawns | Same | |
| Rook files | Open +25, semi-open +12 | Open +18, semi-open +8 | |
| Outpost knight | +15 | +10 | 5th/6th rank, protected by own pawn, unreachable by enemy pawns |
| Rook behind passer | Bonus for rook on same file as passer (own or enemy) | Same | |
| King-passer proximity | Bonus for king distance to passers | Higher weight in endgame | |

### Material phase blending

The game phase is computed as:
```
phase = pawns×0 + knights×1 + bishops×1 + rooks×2 + queens×4
max_phase = 24
score = (mg × phase + eg × (max_phase - phase)) / max_phase
```

This smoothly transitions from midgame to endgame evaluation as pieces are exchanged.

### Tunable parameter struct

All weights live on `Eval` as public fields, enabling:
- Runtime tuning via UCI options (future)
- A/B testing of parameter changes without recompilation (future)
- `Eval::default()` returns the PeSTO baseline

The `Eval` struct is constructed once and cached in a `LazyLock<Eval>` static. The public `EVAL.evaluate(board)` call uses this default, eliminating per-call heap/stack allocation overhead.

### Static Exchange Evaluation (SEE)

SEE lives in `eval.rs` alongside the evaluation function because it shares the piece value constants and attack infrastructure. SEE is a recursive capture simulation on a target square, using the smallest attacker first, to determine if a capture wins or loses material. It is used by:
- **Move ordering**: winning captures (SEE > 0) get top priority; losing captures (SEE ≤ 0) are deprioritized but still searched
- **Quiescence search**: captures with SEE < 0 are pruned entirely (losing exchanges don't need quiet resolution)

### Attack module separation

Precomputed attack tables (knight, king, pawn) and magic slider tables live in a separate `src/attack.rs` module. The evaluation and SEE functions import attack primitives from this module, keeping concerns separated.

## Why

- **Tapered eval**: The standard approach for handcrafted evaluations. Midgame and endgame positions require different piece-square tables and positional weights. Linear interpolation between the two is simple, fast, and effective.
- **PeSTO PSTs**: Well-known, open-source tables that provide reasonable positional play out of the box.
- **`LazyLock` static**: Eliminates per-call allocation (previously `Eval::default()` was called on every `evaluate()` invocation, copying ~3KB of stack data). The `LazyLock<Eval>` static initializes once and serves immutable references thereafter.
- **SEE replaces MVV-LVA**: MVV-LVA is a heuristic (victim value × 10 − attacker value) that doesn't account for recaptures. SEE correctly identifies losing exchanges (e.g., Q×R when opponent recaptures with a pawn). This improves both move ordering accuracy and q-search efficiency.

## Considered options

| Option | Rejected because |
|--------|------------------|
| Full PeSTO copy (standard, no tuning) | A flat function with hardcoded numbers is simpler but locks out tuning. The `Eval` struct adds ~40 lines of boilerplate and unlocks parameter tuning later. |
| Separate SEE module (`src/see.rs`) | SEE is ~50 lines of code and tightly coupled to evaluation piece values and attack infrastructure. A separate module would add file overhead for minimal separation benefit. |
| NNUE evaluation | Requires a trained neural network, weight file loading, and SIMD inference. Complexity is 100× higher for a 50-100 Elo gain. Not appropriate for a hobby engine at this stage. |
| Material-only evaluation | The search would find good moves with deep enough search, but positional understanding (pawn structure, king safety) significantly improves move quality at shallow depths. |

## Consequences

- The `Eval` struct is ~3KB in size. The `OnceLock` pattern ensures this is allocated once at startup, not on every evaluation call.
- All evaluation weights are in one place. Adding a new positional term requires: adding fields to `Eval`, adding defaults, adding evaluation logic. The pattern is consistent.
- `evaluate()` is called millions of times per second during search. The hot path avoids allocation and indirection beyond the `OnceLock` read.
- SEE depends on `attack.rs` primitives (knight/king/pawn attacks, magic slider tables). These must be initialized before SEE is used. The `init_slider_tables()` call in `main()` guarantees this.
