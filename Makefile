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

.PHONY: help build window run headless batch test check review fmt clippy clean decisions decisions-check next-step-check plan-check ai-review

help:
	@echo "make build      - build everything, release"
	@echo "make run        - play: opens a window on the simulation (needs a display)"
	@echo "make headless   - run the acceptance match with no window (SEED=$(SEED), TICKS=$(TICKS))"
	@echo "make batch      - the acceptance match across seeds (SEEDS=$(SEEDS))"
	@echo "make test       - every test"
	@echo "make check      - fmt + clippy + tests: the pre-commit gate"
	@echo "make review     - the review gate: does the staged change carry its review?"
	@echo "make ai-review  - the same change through the AI reviewer (OCR_ARGS=--commit HASH by default HEAD)"
	@echo "make decisions  - rebuild the browsable decision index (docs/site/index.html)"
	@echo "make plan-check - is docs/dependency-tree.md consistent, and is every open item scheduled?"
	@echo "make clean      - remove target/"

build:
	$(CARGO) build --release --workspace

window:
	$(CARGO) build --release -p ruage-view --bin window

# The game as it stands today: terrain and units as blocks, no sprites yet, no HUD.
run: window
	./target/release/window $(CONFIG) --seed $(SEED)

headless:
	$(CARGO) build --release -p ruage-harness --bin ruage-headless
	./target/release/ruage-headless $(PACK) --script $(CONFIG)/acceptance.toml \
		--seed $(SEED) --ticks $(TICKS) --report $(REPORT)

batch:
	$(CARGO) build --release -p ruage-harness --bin ruage-batch
	./target/release/batch $(PACK) --script $(CONFIG)/acceptance.toml --seeds $(SEEDS)

# **A test run without the generated data proves nothing**: every data-driven suite skips itself, so `cargo test` reports
# success while running almost nothing. This gate says so out loud (tools/require_data.py, and D8 for the polite skip).
require-data:
	python3 tools/require_data.py

test: require-data
	$(CARGO) test --workspace

# The decision index (D168): one browsable page over docs/decisions.md and the U-register. Committed, because the
# corpus it renders is committed. `make decisions-check` parses and reports without writing.
decisions:
	python3 tools/decisions.py

next-step-check:
	python3 tools/next_step.py --check

# The dependency tree (docs/dependency-tree.md): every id defined once, every open item named by a phase, every citation
# pointing at a file that exists and a line inside it. Its own tests are in tools/test_plan_check.py.
plan-check:
	python3 tools/plan_check.py

decisions-check:
	python3 tools/decisions.py --check

# The tools' own tests, including the gate above. Cheap, and they are what keeps a tool honest.
tools-test:
	python3 -m unittest discover -s tools -p 'test_*.py'

check: fmt clippy test tools-test next-step-check plan-check
	@echo "check: clean"

fmt:
	$(CARGO) fmt --all -- --check

clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

# The review gate, runnable by hand; `.githooks/pre-commit` runs the same script and refuses the commit.
review:
	python3 tools/review_check.py

# The AI reviewer (open-code-review, the `ocr` CLI): an input to the adversarial review, never a replacement for it. It
# reads the diff and the files around it and answers in findings - it cannot run the probe, so the numbers in the note
# are still ours, and a finding is a hypothesis until one of us checks it.
#
# Tuned for this box: 2 CPUs and a metered endpoint. The tool's own defaults (max effort, 8 workers, no ceiling) spent
# **1,438,799 tokens and 15m43s** on a 33-line commit; low effort with two workers is the setting to compare against
# before raising either. The JSON lands in build/, which is gitignored: a note quotes the run, the repo does not carry it.
OCR ?= ocr
OCR_ARGS ?= --commit HEAD
OCR_EFFORT ?= low
OCR_CONCURRENCY ?= 2
# A cost stop, not a completion target. Measured on the same 33-line commit: kimi-k2.7-code finished inside 410k;
# deepseek-v4.1-flash spent 551,573 at a 400k ceiling, 862,827 at 700k and 1,355,935 at 1.2M, and **never finished** - each
# ceiling was overrun and two or three of eight groups were cut with `classification: budget`. Raising it bought tokens,
# not completeness, so it is pinned where a run is affordable and the parts that were cut are printed below.
OCR_BUDGET ?= 700000
# It excludes Markdown by extension (as `unsupported_ext`) but *not* `.toml` or `.html`: a docs-only commit of mine cost
# **236,189 tokens and 3m21s** reviewing a TOML and the 333 KB generated `docs/site/index.html`, and returned nothing.
OCR_EXCLUDE ?= docs/**,build/**,target/**,config/gamedata/**,**/*.html

ai-review:
	@command -v $(OCR) >/dev/null || { echo "ai-review: no '$(OCR)' on PATH - npm i -g @alibaba-group/open-code-review @alibaba-group/ocr-linux-x64, then see D209"; exit 1; }
	@mkdir -p build
	$(OCR) review $(OCR_ARGS) --effort $(OCR_EFFORT) --concurrency $(OCR_CONCURRENCY) \
		--max-tokens-budget $(OCR_BUDGET) --exclude '$(OCR_EXCLUDE)' --audience agent --format json -o build/ocr-review.json
	@python3 -c "import json;d=json.load(open('build/ocr-review.json'));s=d.get('summary') or {};print('ai-review:',d.get('status','?'),'-',s.get('files_reviewed','?'),'files,',s.get('comments','?'),'comments,',s.get('total_tokens','?'),'tokens,',s.get('elapsed','?'));f=[(i.get('path'),i.get('classification')) for i in (d.get('manifest',{}).get('coverage',{}).get('failed') or [])];print('ai-review: CUT -',', '.join(p+' ('+str(c)+')' for p,c in f),'- those files were not read') if f else None" || echo "ai-review: no run manifest written (preview, or the review did not start)"

# The gate's own tests: the file the gate is written in is the file most likely to rot.
review-test:
	python3 -m unittest discover -s tools -p 'test_*.py'

clean:
	$(CARGO) clean
