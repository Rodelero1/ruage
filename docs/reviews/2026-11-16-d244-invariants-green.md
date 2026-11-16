# D244 - both red invariants fixed, measured, reverted

**Scope.** Documentation only. Two code changes were built, measured and reverted in this session; `crates/sim/tests/invariants.rs`
and `crates/sim/src/` are exactly as D243 left them (`92c88f0`).

## Performance

**Acceptance match, seed 7, 48,000 ticks, release** (gate: entities >= 155, `repaths` near 380; the D237 baseline is 38.36 s and
38.45 s per run on this box):

| build | entities | `repaths` | verdict |
|---|---|---|---|
| HEAD (D243) | **155** | **497** | - |
| rule 3: bank only at the wall | **136** | **28** | rejected: carriers never satisfy the test |
| rule 4: no settle once working | (not run) | - | rejected on `gather_motion`, before the match |

Rule 3's change is a **-12.3%** loss of entities and a **-94%** collapse in re-paths: the villagers stop making trips at all,
which is the economy dying rather than a cost.

## Stability

**Attack 1 - the rule-3 fix is right and the geometry under it is not.** `footprint_gap(position, drop) <= body_radius` is exactly
the owner's sentence, and the fixture turns green - but the match dies, which can only mean the rectangle the test measures
against is not the wall the bodies stand at. `Entity::position` for a building and `footprint_of`'s tile count have not been shown
to agree on the anchor; that is the read this needs, and it was not guessed at.

**Attack 2 - the rule-4 fix switches off a mechanism with two jobs.** Gating `settling` on `progress == 0` makes a working man
stay put (fixture green) and **immediately reddens** `gather_motion::three_gatherers_on_one_bush_do_not_pile_up`: the settle is
also what arranges the ring round a node (D200/D202). One gate cannot separate "arrange me" from "nudge me while I work", which is
the shape the next attempt needs.

**Attack 3 - the fixtures were not bent to make a fix pass.** Rule 4's fixture did get one honest gate: it now judges only
`progress > 0`, the engine's own "he is taking from it now" and the same gate rule 2 uses. Before that it flagged the last few
thousandths of a tile of an *approach* as movement, which would forbid a man from ever arriving. That is a definition of
"gathering", stated in the fixture, not a loosening to get green: with the gate the fixture still fails on the settle step.

**Attack 4 - nothing was left half-applied.** The tree is `92c88f0` plus this documentation; `git status` is clean apart from
these docs, and the acceptance numbers above are from builds that were then discarded, not from a tree that shipped.

**Not attacked, and why.** Neither fix was re-attempted with a tuning knob (a bigger delivery radius, a settle that moves only
every N ticks): both would hide a missing read behind a constant, and the two reads are named instead.
