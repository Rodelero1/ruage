# 2026-11-16 - villagers, overlap and pushing: the research behind the owner's ruling

**What changed.** Documentation only. `docs/movement-in-the-binary.md` gains *"The path ignore list, read (2026-11-16)"* and
`docs/prior-art.md` gains the same-day section of the same name. No `crates/`, `config/`, `tools/` or `build/` file is touched,
no test count moves, and no hash moves: the acceptance match still reports `hash=cb1f6a83a119a2a3 repaths=256 made_room=0` for
`--seed 7 --ticks 24000` in a debug build, which is what it reported before this note existed.

## Performance

**No measurement applies, and none is invented.** A docs-only change cannot move a frame or a tick, and this tree's rule is
explicit that demanding a number from a typo fix teaches everyone to write a fake one. What was measured, for the record, is
the *probe* the note rests on rather than the note: the acceptance run at `--seed 7 --ticks 24000`, 2 min 59.7 s in a debug
build (18.98 s for `--ticks 6000`), dumping 152 entities, from which the overlap census quoted in the reply was taken - 11
pairs of bodies inside the 0.4-tile villager standoff, closest 0.192, and 7 units standing on a node's own tile.

## Stability

What was attacked, since a research note's failure mode is a *wrong fact* rather than a crash:

1. **Every load-bearing quote re-read at the source, not taken from the researcher's summary.** openaok
   `src/ecs/systems/MovementSystem.cpp:127-129` and `:243-247`, `src/ecs/Navigation.cpp:230`; genie
   `pathing/mod.rs:343-360`, `movement/walk.rs:92`, `objects/release.rs:209-223`;
   `doc/reverse_engineering/game_mechanics/formations.md` lines 40 and 206 fetched fresh from `raw.githubusercontent.com`.
   The clones were shallow (`depth 1`), so the only hash the note asserts is openaok's clone head `95cb8a0` - upstream's head at
   clone time, not a hash from the project's own log - and the quotations are the evidence, being line-exact.
2. **The binary claims cross-checked against a second reading of the same data.** `+0x2a0` being `isMoving` is asserted twice
   by two different routes: the vtable arithmetically (`0x9e82c0 + 0x2a0 = 0x9e8560`, whose entry is `0x8aa290`, which returns
   whether the velocity vector at `+0x130` is non-zero) and the collision log's own `isMoving=%d` argument. The neighbouring
   slots agree with `docs/movement-in-the-binary.md`'s earlier reads (`+0x2ac` = `0x4b0860`, `+0x2b0` = `0x8aa6d0`), so the
   vtable base is not in doubt.
3. **The claim most likely to be wrong, stated as open.** Nothing in the note asserts what fills route `+0x1a`/`+0x1b`; the
   formation reading comes from openage's notes, not from our own binary. If a later read shows HD writing that array for a
   villager-only group order, the *narrowing* recommended here is wrong and D207's original scope was right. The note says so
   in its own words rather than in this one.
4. **Version scoping.** The Forgotten Empires slide's glossary separates legacy `AoE2`, `(AoE2) HD Edition` and `AoE2DE`, so
   its *"No pushing allowed"* line is recorded as engine-lineage evidence, not as proof about HD. The DE-only mechanisms
   (Collide & Correct, 2021-2023; villager stacking removed 2025) are labelled DE in the note.
5. **Licence.** genie is AGPL-3.0: read, quoted in fragments, nothing copied. openaok is MIT; openage is GPL-3.0 and only its
   *documentation* was read for facts. No code entered this tree from any of them, and the clones live under
   `/opt/data/cache/scratch/`, outside the repository.

**Not done, and named rather than hidden:** no test was written, because no behaviour changed. The two tests the ruling owes -
a group of villagers ordered to one tile must not overlap, and a newcomer joining three gatherers must not move them - are
the *next* slice's, and they belong with the narrowing, not with this note.
