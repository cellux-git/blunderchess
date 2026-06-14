# 08 — Extract SearchAlgorithmParams from hardcoded magic numbers

**Category:** improvement
****Status:** completed
**ADR:** docs/adr/0008-search-algorithm-params.md

## Summary

The search module (`src/search.rs`) has 15+ algorithmic tuning parameters as hardcoded magic numbers scattered across `alpha_beta()` and `search_worker()`. Extract them into a `SearchAlgorithmParams` struct with nested sub-structs.

## Sub-structs

- **LmrConfig** — `min_depth`, `min_moves_searched`, `reduction: [u8; 3]` (for 3+/5+/8+ moves searched)
- **NullMoveConfig** — `min_depth`, `r_shallow`, `r_deep`, `deep_threshold`
- **AspirationConfig** — `initial_delta`, `depth_threshold`
- **FutilityConfig** — `max_depth`, `margin_d1`, `margin_d2`

Plus top-level fields: `soft_time_ratio: f64`, `check_extend_qs_depth: u8`.

## Magic numbers to extract

| Location | Value | Field |
|----------|-------|-------|
| LMR depth threshold | `depth >= 3` | LmrConfig.min_depth |
| LMR move threshold | `moves_searched >= 3` | LmrConfig.min_moves_searched |
| LMR reduction steps | `1`, `2`, `3` | LmrConfig.reduction |
| Null-move min depth | `depth >= 3` | NullMoveConfig.min_depth |
| Null-move R | `3` / `4` (depth ≥ 6) | NullMoveConfig.r_shallow / r_deep |
| Aspiration delta | `25` | AspirationConfig.initial_delta |
| Aspiration threshold | `depth >= 4` | AspirationConfig.depth_threshold |
| Futility max depth | `depth <= 2` | FutilityConfig.max_depth |
| Futility margins | `200` / `400` | FutilityConfig.margin_d1 / margin_d2 |
| Soft time ratio | `movetime / 2` | soft_time_ratio |

## Acceptance criteria

- [ ] `SearchAlgorithmParams` struct with nested sub-structs added to `src/search.rs`
- [ ] `Default` impl preserves all current hardcoded values
- [ ] All 15+ magic numbers replaced with `params.field` references
- [ ] Passed alongside `SearchParams` into `search()` → `search_worker()` → `alpha_beta()`
- [ ] `cargo test --lib --test benchmarks` passes cleanly
- [ ] No behavioral change (identical search output at each depth)
- [ ] No NPS regression
