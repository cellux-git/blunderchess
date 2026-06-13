# Opening book: Polyglot format

**Status**: accepted

## Context

A chess engine benefits from an opening book to play strong, varied opening moves without search. The book is a database of position → move mappings with weights. The engine needs to query it quickly at the root of search.

## Decision

The engine uses the **Polyglot opening book format** (`.bin` files). Each entry is a 16-byte record: 8-byte Zobrist hash key, 2-byte move, 2-byte weight, 4-byte learn. Lookup is a binary search over the sorted hash keys, yielding a list of candidate moves for weighted random selection.

## Why

- Polyglot is the de facto standard for chess opening books. Every major GUI (Arena, CuteChess, Banksia) and book-authoring tool (SCID, Polyglot itself) produces and consumes this format.
- Interoperability: users can download community-curated `.bin` files (e.g. from the Cerebellum or Goi projects) and use them directly — no conversion needed.
- The format is simple: fixed-width records, sorted by hash, binary-searchable. A loader is ~60 lines of Rust with zero dependencies.
- Binary search provides O(log n) lookup with negligible overhead at game start.
- Weighted random selection from matching entries provides move variety (stronger than always playing the top-weighted move).

## Considered options

| Option | Rejected because |
|--------|------------------|
| No opening book | The engine would search from move 1, missing centuries of opening theory. Blunder-prone in the opening phase, especially at shallow depths. |
| Custom binary format | No tooling support. Users would need a bespoke converter. Why invent a format when Polyglot exists? |
| PGN-based book | Querying requires full-text search over millions of positions. Impractical for a lightweight engine — either slow or requires an index that essentially becomes Polyglot. |
| Embedded opening lines in code | Inflexible, unmaintainable, and limited to whatever the author hardcodes. |

## Consequences

- UCI options `OwnBook` (enable/disable) and `BookFile` (path to `.bin`) control book usage.
- The book is consulted once at the root — if it returns moves, search is bypassed for the first move.
- The engine does not learn from games or update the book file; it is read-only.
- Changing book formats would require rewriting `src/book.rs` and the UCI option handling, but the interface (`fn probe(hash) -> Option<Vec<(Move, weight)>>`) would remain similar.
