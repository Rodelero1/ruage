# A thin wrapper around cargo, not a build system: cargo *is* the build system, and duplicating its dependency graph in make
# would be a second source of truth that drifts. What make is genuinely good for here is the two things cargo cannot know:
# the long list of config files the binaries take, and the difference between "the window" and "the balance harness".
#
#   make            the same as `make help`
#   make run        the game, in a window (needs a desktop session)
#   make headless   the acceptance match, no window, with a report every 400 ticks
#   make batch      the acceptance match across seeds, one row each
#   make check      fmt + clippy + every test: what has to pass before a commit

CARGO ?= cargo
CONFIG ?= config
SEED ?= 7
TICKS ?= 48000
REPORT ?= 400
SEEDS ?= 7

# Every binary in the workspace reads the same pack; this is the one place that list lives.
PACK = --config $(CONFIG)/sim.toml \
       --map $(CONFIG)/map.toml \
       --economy $(CONFIG)/economy.toml \
       --units $(CONFIG)/gamedata/units.toml \
       --resources $(CONFIG)/resources.toml \
       --combat $(CONFIG)/combat.toml \
       --techs $(CONFIG)/gamedata/techs.toml \
       --effects $(CONFIG)/gamedata/effects.toml \
       --civs $(CONFIG)/gamedata/civs.toml

.PHONY: help build window run headless batch test check fmt clippy clean

help:
	@echo "make build      - build everything, release"
	@echo "make run        - play: opens a window on the simulation (needs a display)"
	@echo "make headless   - run the acceptance match with no window (SEED=$(SEED), TICKS=$(TICKS))"
	@echo "make batch      - the acceptance match across seeds (SEEDS=$(SEEDS))"
	@echo "make test       - every test"
	@echo "make check      - fmt + clippy + tests: the pre-commit gate"
	@echo "make clean      - remove target/"

build:
	$(CARGO) build --release --workspace

window:
	$(CARGO) build --release -p ruage-view --bin window

# The game as it stands today: terrain and units as blocks, no sprites yet, no HUD.
run: window
	./target/release/window $(CONFIG) --seed $(SEED)

headless:
	$(CARGO) build --release -p ruage-sim --bin ruage-headless
	./target/release/ruage-headless $(PACK) --script $(CONFIG)/acceptance.toml \
		--seed $(SEED) --ticks $(TICKS) --report $(REPORT)

batch:
	$(CARGO) build --release -p ruage-sim --bin batch
	./target/release/batch $(PACK) --script $(CONFIG)/acceptance.toml --seeds $(SEEDS)

test:
	$(CARGO) test --workspace

check: fmt clippy test
	@echo "check: clean"

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

clean:
	$(CARGO) clean
