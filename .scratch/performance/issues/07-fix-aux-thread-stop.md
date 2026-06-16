# 07 — Fix auxiliary thread stop signal in Lazy SMP

**Status:** `completed`

**Status**: `completed`
**Category**: `bug`

## Problem

In `search_mt()` (search.rs ~114), auxiliary threads (tid ≥ 1) create a local `AtomicBool::new(false)` for their stop signal. This local `AtomicBool` is never set to `true`, so auxiliary threads never stop when the main I/O thread sets the real stop flag. They run to completion of their current depth iteration, over-searching.

## Solution

Give ALL threads a reference to the **same** `Arc<AtomicBool>` stop signal:

```rust
let stop = Arc::clone(&stop); // shared, not per-thread
let t_stop = Arc::clone(&stop);
thread::spawn(move || {
    search_worker(&board, &params, i, &t_stop, &tt);
});
```

Remove the `if tid == 0` branch that creates a separate local bool. All threads share the same stop signal.

The search already handles stop correctly inside `should_stop()` — the issue is only that auxiliary threads don't see the flag.

## Acceptance criteria

- [ ] All threads share the same `Arc<AtomicBool>` stop signal
- [ ] `stop` command sets the flag and all threads stop within their current ply
- [ ] All 86 unit tests pass
- [ ] All 10 tactical tests pass
- [ ] Multi-threaded search test passes (search.rs test_search_multi_threaded)

## Comments
