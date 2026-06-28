Agent workflows: see `docs/agents/` (issue-tracker, triage-labels, engineering-skills, testing).

## Domain docs

Single-context: `CONTEXT.md` + `docs/adr/` at the repo root.

## Performance constraints

- Changes must not cause performance degradation on hot paths without explicit approval. Hot paths include: search (`alpha_beta`, `quiescence`), eval, movegen, `make_move`/`unmake_move`, TT probe/store, move ordering, and attack/detection (`is_attacked_by`, `compute_pinned_impl`).
- If a change might affect hot-path performance, run the release benchmarks before and after. Compare delivered NPS against the baseline in `docs/performance.md`. If NPS drops measurably (more than noise), flag it — do not proceed without user confirmation.
- Zero-compile-time changes (dead code removal, visibility tightening, import cleanup, attribute removal) are presumed safe. Verify with `cargo check` and the full test suite.
