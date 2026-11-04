# A camp where a drop site already serves the wood is a wasted camp

Owner's note, 2026-11-04: *"Lumber camps should not be built too close to the existing wood drop site (another lumber camp
and town center)."*

I asked before choosing a number, because "too close" is exactly the kind of phrase that a plausible default would invent an
answer for. The answer is in the reference implementation, and it is not a guess:

- `src/ai/Placement.cpp:41-44` (OpenAoK, MIT) — *"**The nearest resource to `turn` not already covered by a drop site for
  `type`** ... `kCovered = 5`: a drop site already this near a forest, mine or forage makes another there pointless."* So the
  spacing is **5 tiles**, measured from the resource to the drop site.
- `src/ai/Placement.cpp:134` — the camp must also be within `sn-camp-max-distance` of home, and `src/ai/Language.cpp:291`
  gives that strategic number its default of **25**.

So: 5 and 25, both as content (`[build] drop_site_spacing`), both cited where a reader will find them.

## What changed

- `World::drop_site_within(player, variant, at, tiles, town)` — the predicate, asked the way the AI asks it: it takes the
  worker's own variant, so the drop sites that count are the ones *that worker may hand a load in at* (the dat's
  `drop_sites` edge, 109/68 for a villager), not a class or a role name.
- `World::build_spaces_drop_sites(role)` — which roles must not be built where it is already served, from content
  (`[build] spaced_roles`). The owner named lumber camps; a mine can join the list without the engine learning either word.
  A name that is not in `[role]` is a **load error**, not a silently inert rule.
- `script.rs`, in both build resolvers — the node choice now walks the nearest-first list and takes the first node that is
  *not* covered, up to sixteen candidates, and refuses with `no {resource} node that a drop site does not already serve`
  rather than paying for a camp that saves nobody a walk.

## One deliberate difference from the reference, and why

OpenAoK passes `town = false` for a lumber camp (`Placement.cpp:129`), i.e. **its camp rule does not count the town centre**
as covering wood. Our owner asked for the opposite in as many words — *"another lumber camp and town center"* — and the
player's expectation agrees with him: a camp beside a town centre that already accepts wood buys nothing. So the camp rule
passes `town = true`, a mill passes `town = false` (a mill must be free to stand at the berries the town centre sits beside),
and the difference is written down here rather than left for someone to rediscover as a bug.

## The test, written first

`crates/sim/tests/drop_spacing.rs` — one test, three claims, all three failing before the implementation existed:

- wood three tiles from the town centre **is** covered (`true`);
- wood eighteen tiles away is **not** (`false`);
- the same near wood is **not** covered when the caller excludes the town centre, which is the mill's question.

The first version of this file failed for a reason worth recording: I passed the *entity's* `variant` field, which is `None`
on a seeded villager (the variant **is** its unit id, 83), so the predicate answered "no drop sites at all" and both claims
came back false. The probe that found it printed the villager's `drop_sites` as `[109, 68]` and the predicate as `true` for
the same tile — the test was asking a different question than the probe.

## Measured

- **620 passed / 0 failed / 18 ignored** across the workspace.
- `clippy --workspace --all-targets -D warnings`: clean; `cargo fmt --all` applied.
- Acceptance, 36,000 ticks, seed 7: `tick=36000 entities=162 players=1 hash=13f09cf0930da590 repaths=119 made_room=174` —
  the match still wins, and **no refusal ever says "no wood node that a drop site does not already serve"**, so the rule costs
  no camp it needs. The one camp in the final board stands at (21, 12), 6.4 tiles from the town centre at (16, 16): outside
  the 5-tile ring, which is what the rule asks for.

## Performance

The rule costs **nothing measurable**. The predicate is one scan of the buildings list (tens of entries) times a table
lookup, and it runs only when a camp rule fires - nine times in a 36,000-tick match, at most. The site search itself is
unchanged in the common case: the first candidate is almost always uncovered, so the loop exits on `nth = 0`.

Measured end to end, same pack and seed, 36,000 ticks: **162 entities, players=1, hash=13f09cf0930da590, repaths=119,
made_room=174** - identical shape to the run before the change, with the match still won. The wall-clock of the whole run is
dominated by pathfinding (29 s at 36,000 ticks), and the rule moves it by nothing this harness can resolve.

## Stability

The predicate is a scan over buildings (tens) times a table lookup, called once per camp rule firing — it cannot reach a
frame. The candidate walk is bounded at sixteen, so a map whose wood is all covered refuses in bounded time instead of
searching forever. The 25-tile range is **not** enforced yet: a camp may still be placed at a far forest if that forest is
uncovered. That is the next half of the rule, and it needs the camp search to prefer covered-but-near over uncovered-but-far,
which is a decision about which is worse rather than a number.
