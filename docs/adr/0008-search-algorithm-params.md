# Search algorithm parameters as nested sub-structs

**Status**: accepted

## Context

The search module (`src/search.rs`) contains 15+ algorithmic tuning parameters as hardcoded magic numbers scattered across `alpha_beta()` and `search_worker()`: LMR reduction values (1/2/3 steps), null-move R (3/4), aspiration window delta (25), futility margins (200/400), soft time-limit ratio (0.5), and depth thresholds for each heuristic.

`SearchParams` only holds UCI-level options (depth, movetime, threads). Tuning any algorithm parameter requires source edits in multiple functions. No test can verify behavioral changes from parameter adjustments (e.g., "with more aggressive LMR, the engine reaches depth N in fewer nodes").

## Decision

Add a `SearchAlgorithmParams` struct with nested sub-structs for each search heuristic:

- **LmrConfig** — `min_depth: u8`, `min_moves_searched: u8`, `reduction: [u8; 3]` (for 3+, 5+, 8+ moves searched)
- **NullMoveConfig** — `min_depth: u8`, `r_shallow: u8`, `r_deep: u8`, `deep_threshold: u8`
- **AspirationConfig** — `initial_delta: i32`, `depth_threshold: u8`
- **FutilityConfig** — `max_depth: u8`, `margin_d1: i32`, `margin_d2: i32`

Plus top-level fields for heuristics that don't warrant their own sub-struct: `razor_margin: i32` (razor pruning threshold), `soft_time_divisor: u64`.

`SearchAlgorithmParams` implements `Default` with the current hardcoded values. It is passed alongside `SearchParams` into `search()` and threaded through to `search_worker()` and `alpha_beta()`.

## Why

- **Locality**: all 15 tunables live in one struct. A tuning session edits one file, not seven function bodies.
- **Testability**: can construct "aggressive LMR" and "conservative LMR" param sets and compare node counts at fixed depth.
- **Leverage**: one struct gates the entire search algorithm's behavior.
- **Deletion test**: remove `SearchAlgorithmParams` — the 15 constants must live somewhere; they'd scatter back into the search code.

## Considered options

| Option | Rejected because |
|--------|------------------|
| Hardcoded magic numbers (current) | No locality. Tuning requires grep-and-replace across multiple functions. No testability. |
| Flat struct (one field per knob) | 15 top-level names with no grouping. LMR fields (`min_depth`, `min_moves`, `r1`, `r2`, `r3`) belong together; a flat struct scatters them alphabetically. |
| One giant params struct (merge with SearchParams) | SearchParams is UCI-facing (serialized from `go` command). Algorithm params are engine-internal. Mixing them couples protocol and algorithm. |

## Consequences

- `alpha_beta()` signature gains a `&SearchAlgorithmParams` parameter (or reads from a const reference).
- All magic numbers are replaced with `params.lmr.reduction[step]` etc.
- Default impl preserves current behavior exactly — no search behavior change on merge.
- Future heuristics (e.g., singular extensions) add their config as a new sub-struct.
