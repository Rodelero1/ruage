# Carrying four in-flight files so HEAD builds

`339de01` (D248) was pushed and does **not** compile on its own: `crates/sim/src/lib.rs` calls `footprint_centre`, which lives in
an uncommitted change to `crates/sim/src/world/sites.rs` that was in the tree from a **concurrent session**. Found by stashing
those four files and running the gate on the pushed tree alone - clippy stopped at **E0599** (`no method footprint_centre`).

The four files are **not my work and have not been reviewed here**: `world/build.rs`, `world/research.rs`, `world/sites.rs`,
`tests/card_age.rs`. They are committed so that HEAD builds and the gate that was measured is the gate that shipped.

## Performance

The full gate with these files in place ran in **~680 s** and reported **900 passed, 0 failed, 22 ignored**; the 48,000-tick
acceptance run took **45,617 ms** (`entities=137 repaths=710 hash=ac2e785d9f551b55 made_room=0`). No A/B pair was taken for these
four files, so no percentage is claimed for them.

## Stability

**Attack 1 - a commit that does not build is worse than no commit.** The check that found it was running the gate on the pushed
tree with the carried files removed; that is now the rule for any commit that mixes measured work with someone else's tree state.

**Attack 2 - the carry is labelled, not hidden.** The commit message and this note both say the four files are unreviewed and whose
they are, so the next reader does not attribute them to D248.

**Not attacked**: whether those four files are correct on their own terms - that belongs to whoever wrote them.
