# blunderchess

A UCI chess engine written in Rust.

## Build

```bash
# Release (for play)
cargo build --release

# Dev (debug info, faster compile)
cargo build
```

The binary is `target/release/blunderchess` (or `target/debug/blunderchess`).

## Test

```bash
cargo test --lib                    # unit tests (fast)
cargo test --test benchmarks        # integration tests
cargo test --lib --test benchmarks  # all tests
```

## Run

Paste the binary path into your GUI (Arena, CuteChess, Nibbler) as a UCI engine.

Or pipe commands directly:

```bash
echo -e "position startpos\ngo depth 8\nquit" | target/release/blunderchess
```

## Key options

```
setoption name Hash value 64        # MB (default 64)
setoption name Threads value 4      # CPU threads
setoption name MultiPV value 3      # multi-PV analysis
setoption name OwnBook value true
setoption name BookFile value /path/to/book.bin
```
