# ruage

A from-scratch, deterministic reimplementation of **Age of Empires II HD 5.7**: a fixed-point
simulation with seeded RNG and stable iteration order, driven by a content pack the game's own data
is imported into. It reads *your* legally purchased install, in place — no game data is ever copied,
converted for redistribution, or committed.

The engine is for a game, not for Age of Empires II (D17). It knows *kinds*, never particular ones:
resources are columns with names your content pack gives them, costs name their resource rather than
quoting an id, and any number that means something only because of AoE2 lives in the importer. That
is what makes an original content pack possible without touching engine types — and what the test
`a_content_pack_with_our_own_resources_loads_and_spends` proves.

## What works today

| | |
|---|---|
| Fixed-point, deterministic tick loop | `crates/fixed`, `crates/sim` — exact per-tick displacement, PCG32 with separate sim/map streams, FNV-1a state hash |
| Content pack + strict loading | `crates/data` — `sim`, `economy`, `map`, `resources`, `units`, `techs`, `effects`, `combat`, with a content hash for the match handshake |
| A match | seeded from the map and economy, villagers gather/deposit, A\* pathing, build, train, combat, and a scripted ten-minute scenario |
| Resources as content | names, slots, optional reference ids, and where a node may stand — the enum that used to be the pack is gone |

Not there yet, deliberately: a renderer, tech *effects* being applied (they load and are censused,
kinds 4/5 are pinned but not interpreted), and formations.

## Layout

```
crates/       the engine (Rust): fixed-point arithmetic, config loading, simulation
config/       the content pack (our schema). gamedata/ is generated, and gitignored
docs/         decisions, mechanics, the config layer, licensing
SPEC.md       the entry point: state, decisions, procedure, test-first rules
```

`config/gamedata/*.toml` is **generated** from your install by a separate program, in its own
repository: [`ruage-tools`](https://github.com/Rodelero1/ruage-tools) (GPL-3.0-or-later, because it
imports the GPL `.dat` parser). The engine never reads a `.dat`; it reads the TOML that importer
writes, which is why this repository can stay permissively licensed while the importer cannot.

```bash
# 1. generate the content pack into this checkout
git clone https://github.com/Rodelero1/ruage-tools ../ruage-tools
cd ../ruage-tools
uv run python -m genconfig /path/to/Age2HD/resources/_common/dat/empires2_x2_p1.dat \
                           ../ruage/config

# 2. run the engine's tests and the headless match
cd ../ruage
cargo test
cargo run --bin ruage-headless -- --config config/sim.toml --map config/map.toml \
  --economy config/economy.toml --units config/gamedata/units.toml \
  --resources config/resources.toml --combat config/combat.toml \
  --script config/scenario.toml --seed 7 --ticks 200 --report 10
```

The headless runner prints `tick=… entities=… players=… hash=…`; two runs with one seed must print
the same hash, which is the acceptance test for determinism.

## Licence

**MIT OR Apache-2.0**, your choice (`LICENSE-MIT`, `LICENSE-APACHE`). The content *schema* in
`config/` is under the same terms.

What is **not** covered, because it is not here: the game's data (never committed — see
`docs/licensing.md`), the importer (GPL-3.0-or-later, separate repository), and any art (the engine
points at the install's graphics; original art would be CC-BY-SA, D21).

## Build and run

Needs a Rust toolchain (1.99 or newer; `rustup` is the usual way to get one) and nothing else - no GPU, no system
libraries, no game install for the simulation to run.

```sh
make            # what the targets are
make run        # the game, in a window (needs a desktop session)
make headless   # the acceptance match, no window, with a report every 400 ticks
make batch      # that match across seeds, one row each
make check      # fmt + clippy + every test: the gate before a commit
```

Or without make, the same things:

```sh
cargo run --release -p ruage-view --bin window -- config        # the window
cargo test --workspace                                          # the tests
cargo run --release -p ruage-sim --bin ruage-headless -- ...     # see `make headless` for the full argument list
```

The window is **blocks, not sprites**: terrain, units and buildings are drawn as coloured shapes, there is no HUD yet, and
the art extraction is the next items in `docs/next-step.md`. Left click selects, right click orders, Space pauses, `1`-`4`
set speed, Escape quits.

On Linux the window needs a session (`DISPLAY` or `WAYLAND_DISPLAY`); a headless server can still run `make headless` and
`make test`, which is where balance and AI work happen.
