# 05 — Piece role evaluation

**Status:** `completed`

**Status**: `completed`
**Category**: `enhancement`

## The position

```
1rb3k1/ppbp1p2/4r2p/2PB4/1PN5/P5P1/1B5P/R5K1 b - - 4 24
```

Black to move. The engine played **...Re8**, a blunder that loses to **Rf1!** (white threatens Rf8# or wins material). Static eval confirms the gap: score is +416 before Re8 but **-431** after — the engine thinks black improved by 850cp by making the losing move.

The root cause: the engine cannot recognize that black's position is already poor because its pieces are passive:
- **Ra8** on a closed file with zero lateral mobility
- **Bc7** blocked behind its own dark-square pawn chain (b7, d7), with zero safe squares

## What to implement

Six evaluation terms across all piece types, each with a focused `eval_*` method, called from `evaluate_side`:

### 1. Rook on closed file (penalty)

For each rook, if the rook's file contains a friendly pawn (the file is not open or semi-open), apply a penalty.

```rust
pub rook_closed_file_penalty: (i32, i32), // mg, eg
```

Suggested default: `(-15, -20)` — worse in endgames when rook activity matters more.

### 2. Bad bishop (generalized)

Replace the current corner-only `eval_trapped_bishops` with a generalized bad-bishop detector. A bishop is "bad" when it's on the same color as multiple own pawns that block its diagonals and have low mobility. Distinguish from temporarily-blocked bishops: if the blocking pawn can advance, the penalty is smaller or zero. A pawn that is fixed (enemy pawn directly in front, or backward) creates a durable blockade.

```rust
pub bad_bishop_penalty: (i32, i32), // mg, eg — per blocking same-color pawn in front
pub bad_bishop_fixed_multiplier: i32, // multiplier when blocking pawn is fixed (e.g. 2x)
```

### 3. Rook on 7th/2nd rank (bonus)

Classic positional bonus: a rook on the opponent's second rank (rank 7 for white, rank 2 for black).

```rust
pub rook_seventh_rank_bonus: (i32, i32), // mg, eg
```

Suggested default: `(30, 40)` — valuable in both phases, slightly more in endgames.

### 4. Rook-queen battery (bonus)

When the queen and a rook share the same file or same rank, apply a battery bonus. Scale by line openness: open file (both sides have no pawns) = full bonus; half-open (only enemy pawns gone) = medium; closed but the blocking piece can move away = small bonus. If the shared line is completely blocked by immovable pieces, no bonus.

```rust
pub rook_queen_battery_bonus: (i32, i32), // mg, eg — base bonus
pub rook_queen_battery_open_multiplier: i32, // e.g. 3x for open, 2x for semi-open, 1x for closed-movable
```

### 5. Queen multi-attack (bonus)

Two components:
- **Fork detection**: queen attacks 2+ enemy pieces that are undefended or higher value. Apply `queen_fork_bonus`.
- **Attack count**: bonus scales with number of distinct enemy pieces the queen attacks (like mobility but counting attacked pieces, not empty squares). Even pressuring defended pieces has eventual value.

```rust
pub queen_fork_bonus: (i32, i32), // mg, eg — bonus when queen forks 2+ pieces
pub queen_attack_count_bonus: [i32; 8], // per distinct enemy piece attacked (capped at 7)
```

### 6. Knight passivity (penalties)

Three sub-terms:
- **Rim penalty**: knight on a-file or h-file gets a penalty.
- **Trapped knight**: knight with zero safe squares (cannot move without capture or onto own piece) gets a penalty.
- **Knight redundancy**: two knights defending each other get a penalty, **waived** if at least one knight is on an outpost (protected by own pawn, not attackable by enemy pawns).

```rust
pub knight_rim_penalty: (i32, i32),
pub knight_trapped_penalty: (i32, i32),
pub knight_redundancy_penalty: (i32, i32),
```

## Key interfaces

- `Eval` struct — add ~15 new fields for the 6 term groups above
- `eval_rooks` — extend with closed-file detection, 7th-rank, and rook-queen battery
- `eval_trapped_bishops` — rewrite to `eval_bad_bishops`
- New `eval_queen_multiattack` — fork detection + attack count
- New `eval_knights` — rim, trapped, redundancy (rename/expand outpost knight detection)
- `evaluate_side` — call new/updated methods
- `Eval::default()` — add sensible defaults

## Acceptance criteria

- [ ] In the blunder FEN (before Re8), white's static eval ≥ +600 (currently +416)
- [ ] After Re8, white's eval is positive (currently -431)
- [ ] `eval_bad_bishops` detects Bc7 as a bad bishop
- [ ] `eval_rooks` penalizes Ra8 for closed file
- [ ] Rook on 7th/2nd rank gets bonus; same rook on 1st/8th does not
- [ ] Good bishop (central, unblocked) is not penalized
- [ ] Bishop temporarily blocked by an advanceable pawn gets reduced/no penalty
- [ ] Rook-queen battery on open file > semi-open > closed-movable > fully-blocked
- [ ] Queen fork on 2+ undefended pieces triggers bonus
- [ ] Knight on a/h file gets rim penalty
- [ ] Two knights defending each other get redundancy penalty (waived if one is outpost)
- [ ] All 89 existing tests pass
- [ ] No NPS regression (>250K at depth 6 in release)
- [ ] At least 6 new unit tests covering the 6 term groups

## Out of scope

- Endgame mobility tables (issue #01)
- Changing the mobility function signature or asymmetry

## Comments

> *This was generated by AI during triage.*

### Grilling session summary

- "Passive piece" refined to piece-specific role evaluation: does the piece fulfill its natural strategic role?
- Rook: wants open files, 7th rank, batteries with queen. Penalized on closed files.
- Bishop: wants long diagonals, targets on kingside. Penalized when blocked by own fixed same-color pawns. Exception: diagonal may open later if blocking pawn can advance.
- Queen: wants to fork and attack multiple pieces simultaneously. Fork detection + attack count.
- Knight: penalized on rim, when trapped, or when redundant (2 knights defending each other, waived if one is outpost).

---

## Agent Brief

> *This was generated by AI during triage.*

**Category:** enhancement
**Summary:** Add 6 piece-role evaluation terms: closed-file rook, bad bishop, 7th-rank rook, rook-queen battery, queen multi-attack, and knight passivity.

**Current behavior:**
- `eval_trapped_bishops` only detects a1/h1/a8/h8 corners behind b2/g2/b7/g7 pawns.
- `eval_rooks` gives open/semi-open bonuses but no closed-file penalty, no 7th-rank bonus, no battery detection.
- No queen multi-attack or fork detection exists.
- Knights have outpost detection but no rim penalty, trapped detection, or redundancy check.
- In FEN `1rb3k1/ppbp1p2/4r2p/2PB4/1PN5/P5P1/1B5P/R5K1 b - - 4 24`, white eval is +416. After Re8, eval flips to -431.

**Desired behavior:**
Six new evaluation terms, all disabled on black's side (Ra8/Bc7 belong to black):

1. **Closed-file rook (Ra8)**: penalty `(-15, -20)` mg/eg
2. **Bad bishop (Bc7)**: per blocking same-color pawn in front × multiplier if fixed
3. **7th-rank rook**: bonus `(30, 40)` mg/eg
4. **Rook-queen battery**: bonus scaled by line openness (open > semi-open > closed-movable)
5. **Queen multi-attack**: fork bonus when attacking 2+ enemy pieces + per-piece attack count bonus
6. **Knight passivity**: rim penalty, trapped penalty, redundancy penalty (waived if one is outpost)

Blunder FEN eval ≥ +600. After-Re8 FEN eval positive.

**Key interfaces:**
- `Eval` struct — ~15 new fields (6 term groups with mg/eg pairs + multipliers)
- `eval_rooks()` — extend with closed file, 7th rank, queen battery
- `eval_trapped_bishops()` → `eval_bad_bishops()` — generalized detection
- New `eval_queen_multiattack()` — fork + attack count
- New `eval_knights()` — rim, trapped, redundancy (expand existing outpost logic)
- `evaluate_side()` — call new methods
- `Eval::default()` — sensible defaults for all new fields

**Acceptance criteria:**
- [ ] Rook on closed file gets penalty; rook on same file after own pawn removed does not
- [ ] Bad bishop (Bc7) detected and penalized; good bishop is not
- [ ] Bishop blocked by advanceable pawn gets reduced/no penalty
- [ ] Rook on 7th/2nd rank gets bonus; same rook on 1st/8th does not
- [ ] Rook-queen battery bonus scales with line openness
- [ ] Queen fork on 2+ undefended pieces triggers bonus
- [ ] Knight on a/h file gets rim penalty
- [ ] Two knights defending each other get redundancy penalty (waived if one is outpost)
- [ ] Knight with zero safe squares gets trapped penalty
- [ ] Blunder FEN static eval ≥ +600 (currently +416)
- [ ] After-Re8 FEN static eval is positive (currently -431)
- [ ] All 89 existing tests pass
- [ ] No NPS regression (>250K at depth 6 in release)
- [ ] At least 6 new unit tests covering the 6 term groups

**Out of scope:**
- Endgame mobility tables (issue #01)
- Changing the mobility function signature or asymmetry