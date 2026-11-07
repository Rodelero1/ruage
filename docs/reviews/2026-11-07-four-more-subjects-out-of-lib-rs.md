# Four more subjects out of `lib.rs` (D144)

*2026-11-07. The owner: "Please split lib.rs into smaller files. It is still around 10000 lines." Stage 1 of D136 continues —
four more `impl World` subjects moved out verbatim, **10,216 → 8,731 lines**, with the acceptance hash as the proof.*

## What moved

| file | lines | what |
|---|---|---|
| `world/seed.rs` | 475 | `seed_from` (395) and `insert`, the one door every new entity comes through |
| `world/train.rs` | 368 | `check_train`, `start_train`, `spawn_from_building`, `free_spot_near` |
| `world/build.rs` | 284 | `settle_blind_sites`, `cancel_overbuilt`, `cancel_site`, `refund_unit_cost(_part)`, `work_one_tick`, `check_build` |
| `world/research.rs` | 411 | `check_research`, `start_research`, `modifiers_for`, `adjusted`, `max_hp`, `effective_speed`, `finish_research`, `grow_plots`, `apply_tech`, `upgrade_units` |

`lib.rs`: **10,216 → 8,731**. Together with D136's three slices the crate is now `lib.rs` plus **seven** subject files:
`movement`, `memory`, `gather`, `seed`, `train`, `build`, `research`.

## The method, unchanged from D136

1. **Pick a contiguous run** of `impl World` methods. Each of these four was one block — no scattering.
2. **Extract by brace matching**, verbatim, including each method's doc comment; wrap in `impl World { … }`; add
   `pub(crate) mod <subject>;` to `world/mod.rs`.
3. **Let the compiler write the visibility list.** This is where the work is: `insert`, `check_train`, `start_train`,
   `spawn_from_building`, `check_build`, `check_research`, `start_research`, `finish_research`, `adjusted`, `max_hp`,
   `effective_speed`, `work_one_tick`, `settle_blind_sites`, `refund_unit_cost_part` all became `pub(crate)`, and four free
   functions in `lib.rs` (`collect_modifiers`, `add_to_class`, `multiply_class`, `scaled_health`) with them. `TrainPlan`
   became `pub(crate)`; `MAX_PLAYERS` was already `pub`.
4. **Check the seam by hand.** Two orphaned doc-comment risks were read at each cut; both blocks started at a doc comment, so
   the preceding method's comment stayed with its method.
5. **Hash after each pair**, then gate.

## Performance

**No measurable change, and none expected**: this is a pure move. The interesting number is what it *cannot* be — the
compiler sees one crate either way, so cross-module `pub(crate)` calls are inlined exactly as the intra-`impl` calls were.

- **Acceptance match, 36,000 ticks, seed 7**: `f328ebc8d4d34b78`, **141** entities, **188** repaths, **187** made-room —
  identical after **each** of the four moves, checked individually, not only at the end.
- **Release build**: 1m21s after the moves against ~45 s for an incremental rebuild before them — but that is a cold build
  of a rearranged crate, and a second build is 3.6 s. No claim is made from one cold number.
- **Source size**: `lib.rs` 10,216 → 8,731 (−14.5%); the crate's total `.rs` in `src/` grew by ~60 lines, all of it module
  headers and `use` lists.

## Stability

- **No behaviour change by construction, and proved by the hash** rather than by reading 1,500 lines of diff. A move that
  changed behaviour would show up as a different hash; the four checks were identical.
- **The suite is the same size and all green**: 728 passed / 0 failed / 19 ignored, unchanged across the four moves (no
  tests moved, no tests added), clippy `-D warnings` clean, `fmt --check` clean.
- **Two compile-error rounds per file, all of them visibility** — which is the property the recipe relies on: the compiler
  cannot let a *behaviour* slip through a mechanical move, because nothing about the code is edited.
- **The seam defects that no compiler catches** are orphaned doc comments and a doc that describes the old neighbourhood.
  Both were checked at every cut; one file's header was rewritten to say what the subject is rather than "part of lib.rs".

## What is left, and why it is not more of the same

`lib.rs` is still 8,731 lines, and the remaining bulk is a different job:

- **`step` is 1,562 lines** in one function. No file split touches it. That is stage 2 — named phases — and it is the change
  that actually helps a reader.
- `apply` (223) and `validate` (101) belong together as `orders`; `state_hash` (343) + `stamp_*` + `bookkeeping_*` as `state`;
  the combat helpers (`gap_squared_to`, `swing`, `volley`, `true_aim`, `lead_shot`, `blast_victims`) as `combat`; the
  placement helpers (`site_*`, `footprint_area`, `solid_tiles`, `nearest_drop_site`) as `sites`. Those are stage-1 work and
  the next session's start, in that order.
- **`git blame` churn is the real cost** of every slice here: 1,500 lines of history now point at a move commit. The
  mitigation used is the one that keeps the words searchable — every doc comment moved verbatim, so `grep` finds the same
  sentences in the new file.
