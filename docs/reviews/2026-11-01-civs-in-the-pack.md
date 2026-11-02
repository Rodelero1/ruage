# The civ bonuses join the pack identity: cost and proof (2026-11-01)

Scope: `crates/data/src/pack.rs` - `PACK_FILES` gains `civs.toml` (and `pack_path` maps it to `gamedata/`), one test in
`crates/data/tests/pack.rs` (new) and the count in `every_pack_file_is_required` moved 8 → 9. No engine behaviour
changes: the file was already loaded, by the binaries, into the world.

**Why:** the owner asked what stops a player modifying a config for a boost. The answer was "the pack hash refuses a
mismatch" - and then a check showed `civs.toml`, **the civilisation bonuses themselves**, was loaded *outside*
`ContentPack` and therefore outside the hash. A player could raise their own civ's gather rate and pass the handshake.

## Performance

Measured on this container:

| what | number | how |
|---|---|---|
| the extra file's size | **37,812 bytes** | `ls -l config/gamedata/civs.toml` |
| the six pre-existing pack tests, before | **5.76 s** | `cargo test -p ruage-data --test pack` (6 tests) |
| the same six tests, after | **5.91 s** | same command with `-- --skip a_civs_bonuses` |
| FNV-1a over 37,812 bytes | ~**40 us** at 1 GB/s | arithmetic on the hasher, not measured; the 0.15 s above is *run-to-run noise* on a suite that copies the pack to scratch between tests |

The honest reading: one more file in a load that already reads eight, on a hash that is computed once per pack load.
It is not measurable against the noise floor, which is why the number above is stated as noise rather than dressed up as
a cost.

**The pack identity changes for every existing checkout** - the hash now mixes nine files instead of eight, so
`identity()` returns a different 16 hex digits than yesterday. That is the expected cost of adding a file that changes
what the game does, and it is the point: two peers built before and after this commit should not be able to join.

## Stability

- **A missing `civs.toml` now fails the pack load** with the same "is part of the content pack; if it is generated, run
  the generator first" message as the other files, rather than being silently absent from the identity. Pinned by the
  existing `a_missing_file_says_which_one_and_why`, which walks `PACK_FILES`.
- **The count in `every_pack_file_is_required` moved to 9** and its message now names what the extra file is; that test
  exists precisely so a file cannot join the loader without joining the hash.
- **`movement.toml` deliberately stayed out.** Nothing reads it yet (`crates/sim/src/path.rs` mentions it in a comment;
  no loader touches it), and hashing a file nobody consumes would look like protection without being any. It joins the
  list in the commit that reads it, and the reason is written into `PACK_FILES` so the next session does not have to
  rediscover it.

### My own errors, in this slice

- **Written change-first, and named.** The `PACK_FILES` edit came before the test, which is the wrong order for this
  repo (D7). What stands in for the RED is a mutation: with `"civs.toml"` removed from the list the new test **fails**
  (`a_civs_bonuses_are_part_of_the_pack_identity ... FAILED`), and passes when it is put back. That is recorded here
  rather than left implied, per the rule about naming a slice that slipped past test-first.
- **The first version of the test assertion failed `cargo fmt --check`** (a one-line `assert_ne!` that rustfmt wants
  split); the gate caught it before the commit, which is the gate working.

## Not attacked, and named

- **A pack file whose *content* is valid but whose *values* are hostile** is now covered by the identity - but only
  between peers who compare identities. A single-player match has no peer and needs none; the file is the player's own
  game.
- **`gamedata/*` other than the four already listed** (`civs.toml` joins `units`, `techs`, `effects`; nothing else is in
  that directory) - checked by listing the directory; if a fifth generated table appears it must join this list.
- **The handshake itself** is not exercised: `ContentPack::differences` names the disagreeing file (pinned), but the
  refusal is a LAN path that does not exist yet. When it lands, this is the list it compares.
