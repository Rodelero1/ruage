# open-code-review: installed, and used on a real commit before it was trusted

*2026-11-14. D209. Nothing in `crates/` changed: the artefacts are `Makefile` (a new `ai-review` target), `AGENTS.md`
(step 3), `docs/decisions.md`, `docs/decision-areas.toml`, and an install and config outside the repo.*

## What was asked, and what was built

The owner asked for `https://github.com/alibaba/open-code-review` to be installed and used for code reviews. It is an
npm-distributed CLI (`ocr`) that sends a git diff and the files around it to a configured LLM through a tool-using agent,
and answers in file/line findings.

- **Installed without running its postinstall downloader.** `npm install -g @alibaba-group/open-code-review` was refused
  the postinstall script by npm's allow-scripts policy; rather than allow it, the platform package
  (`@alibaba-group/ocr-linux-x64@1.13.9`) was installed beside it, and the launcher resolves the binary from there.
  `ocr version` -> `v1.13.9 (0e9060d) linux/amd64`. Nothing was downloaded from a release URL, and no script ran.
- **On `PATH`** at `/opt/data/home/.local/bin/ocr` (a symlink), because `/opt/data/tools/npm/bin` is not on it.
- **Configured** in `~/.opencodereview/config.json` (mode 600) as a custom OpenAI-protocol provider pointed at the
  opencode-go endpoint with `kimi-k2.7-code`. That endpoint demands an `x-opencode-session` header: without it every
  request is `400 MissingSessionID`, and with any value it returns 200. `extra_headers` is a documented provider field, so
  the config carries `x-opencode-session=hermes-ocr-review` rather than a wrapper proxy. `ocr llm test` -> *connection
  test successful*, *tool-call round trip verified*.
- **Wired as `make ai-review`**, defaulting to `--commit HEAD`, `--effort low`, `--concurrency 2`,
  `--max-tokens-budget 400000`, JSON into gitignored `build/ocr-review.json`, with a one-line summary printed from the
  manifest. `AGENTS.md` step 3 now says to put the step's diff through it before writing the note.

## Performance

Measured on one real commit, `06772fe` ("D207: a group walks over itself, everyone else is solid"), 5 files and 33 lines
of changed code:

| run | settings | files | comments | tokens | elapsed |
|---|---|---|---|---|---|
| 1 | tool defaults: max effort, 8 workers, no ceiling | 9 | 10 | **1,438,799** (1,359,621 in / 79,178 out, 1,211,136 cached) | **15m43s**, 75 tool calls |
| 2 | `--effort low --concurrency 2 --max-tokens-budget 400000`, kimi-k2.7-code | 9 | 7 | **409,607** (369,122 in / 40,485 out, 293,632 cached) | **5m30s**, 21 tool calls |
| 3 | the same, **deepseek-v4.1-flash** (the owner's choice) | 8, 1 cut | 10 | **551,573**, budget exceeded | **5m49s**, 33 tool calls |
| 4 | the same at `--max-tokens-budget 700000` | 8, 2 cut | 11 | **862,827**, budget exceeded | **5m6s**, 47 tool calls |
| 5 | the same at `--max-tokens-budget 1200000` | 8, 3 cut | 11 | **1,355,935**, budget exceeded | **4m45s** |
| 6 | **`--max-tools 50` at 700k** | **8 of 8** | 10 | **763,785** | **5m17s**, 40 tool calls, **complete** |

The first row is the number that matters for how this is used: **1.44M tokens and a quarter of an hour for 33 lines**,
because the agent reads whole files (the crate's `lib.rs` is 234 KB) and iterated 75 tool calls. On 2 CPUs with 8
concurrent subtasks the throughput gain is imaginary anyway, so the target pins two workers and low effort. Both runs are
kept side by side here so the next person can see what the cheap setting buys and what it loses, rather than inheriting a
guess.

Install cost: two npm packages, no compilation, `ocr version` in 0.2 s; `ocr llm test` round trip 7 s including a
reasoning-model reply.

**What the model change cost, and what it bought.** deepseek-v4.1-flash is the better reviewer: on the same commit it
found the missing owner test in `stamp_group` (`step.rs:103`), a wasteful clone on the repath path (`step.rs:1400`), and a
doc comment citing a review file that does not exist, none of which kimi-k2.7-code reported. I checked the first against
the source rather than relaying it - a route's `mates` are what a body stops being solid to, so two players' orders in one
tick would let their units walk through each other - and it is **still unfixed on `main`**.

**The knob that decides whether a review finishes is the tool-round bound, not the token ceiling.** Rows 3-5 raised the
ceiling and stayed `partial` every time; row 6 bounded the rounds at 50 and finished, 8 of 8 groups, at 763,785 tokens.
The groups were not too expensive - unbounded exploration (default 100 rounds) inflated them until the ceiling cut whichever
had not started. That is why `OCR_MAX_TOOLS=50` is now pinned: a complete deepseek pass costs ~764k against kimi's ~410k,
and the ceiling is only a runaway stop. The cut list stays printed, because a `partial` run must never read as a whole-diff
review.

## Stability - what was attacked

1. **A postinstall script that downloads a binary.** npm blocked it and the install still worked, because the platform
   package ships the same binary on npm. Chosen deliberately: a curl-from-releases script is a supply-chain step, and the
   alternative cost one extra `npm install`.
2. **A secret in a config file.** The API key was read from `/opt/data/.env` at configure time, written only to
   `~/.opencodereview/config.json`, and that file is mode 600. It is not in the repo, not in the Makefile, and not in
   this note. **One leak did happen** - see my errors below.
3. **The tree has another writer.** `ocr review` with no arguments reviews the *workspace*: staged, unstaged and
   untracked files. In this tree that would mean sending another session's in-progress code to a third-party model. The
   make target therefore defaults to `--commit HEAD` (`OCR_ARGS` overrides it), and every run in this note was
   commit-scoped for the same reason.
4. **An endpoint that answers 400 for a plain config.** Diagnosed by probe rather than by argument: the same request with
   the header returns 200, with the header recorded as the fix in D209 so nobody re-derives it.
5. **A hook that would make every commit slow.** Deliberately not wired into `.githooks/pre-commit`: one review is
   minutes and a million tokens, and a gate that expensive is a gate people learn to skip with `--no-verify`.
6. **Partial results that exit 0.** `--max-tokens-budget` publishes partial findings and still exits 0 (the tool says so
   itself). The make target therefore prints the manifest's `status` and counts, so "5 comments" is never mistaken for
   "the whole diff was read".
7. **A diff with no code in it still costs a review.** It excludes Markdown by itself (`unsupported_ext`) but not `.toml`
   or `.html`, so reviewing a **docs-only** commit of mine selected a TOML and the 333 KB generated `docs/site/index.html`
   and spent **236,189 tokens and 3m21s** to return nothing. The target now excludes `docs/**`, `build/**`, `target/**`,
   `config/gamedata/**` and `**/*.html` by default: preview on the same commit shows all six changed files excluded and
   the run answers `skipped` in two seconds. `OCR_EXCLUDE=` overrides it.
8. **Reviewing an old commit reads the working tree.** On that same run a tool call failed with
   *`docs/reviews/... exists on disk, but not in 8975a54`* - the agent tried to read a file that the working tree has and
   the commit does not. It is a warning about the mode, not a crash: a commit-scoped review is only trustworthy while the
   tree still resembles that commit, which is why the target defaults to `HEAD`.
9. **A cheap run that quietly reviews less.** Same nine files, but **7 comments against 10 and 21 tool calls against
   75** - and the best finding (the `#[must_use]` drift onto `is_mate`) is in both. So the cheap setting loses the tail of
   the list rather than the head: 3.5x fewer tokens and 2.9x less time for most of the value, which is why it is the
   default and the expensive setting is a thing you reach for deliberately.

## My own errors, in the order they happened

1. **I paid for the same review twice.** The first attempt ran inside `execute_code`, which kills the cell at 300 s — the
   `ocr` child survived the kill and kept calling the model. I only noticed because a second run's session log appeared
   beside the first's. Cost: one partial run (about five minutes of tool calls) plus the full one. The fix used here is to
   launch reviews with `terminal(background=true)`, which is what the target's 15-minute worst case needs anyway.
2. **I printed part of the API key.** `ocr config set custom_providers.ocr-go.api_key "$OPENCODE_GO_API_KEY"` echoes the
   value back masked as `oc_s***coSw`; I did not redirect that output, so the first and last four characters are in the
   transcript of this session. Nothing further was exposed and the key was not rotated, but the correct habit is to send
   every command that touches a key through `>/dev/null` and check the result from the config file instead.
3. **My first two runs reviewed the wrong thing.** `-c 98b3b48` was a docs-only commit, so the tool answered `skipped: no
   items were selected` — which is a correct answer to a pointless question. Reading the manifest's `excluded` list
   (`unsupported_ext`) is what showed it.
4. **I set `--max-tools 25`.** The tool refused it (`below minimum 50, using 50`) and said so on stderr; the run I quote
   above is therefore not the run I asked for. Reported rather than silently retried.
5. **I published a conclusion I had not tested to the end.** After three `partial` runs I wrote that this model "never
   finishes this diff" and made the ceiling a cost stop - without trying `--max-tools`, which is the setting that decides
   it. One run at 50 bounded rounds came back complete on the first attempt. The corrected claim is in D209 and above; the
   lesson is that "I raised the ceiling and it still failed" is evidence about one knob, not about the tool.
6. **I paid three times to learn one thing.** After the 400k run came back `partial`, I raised the ceiling to 700k and then
   to 1.2M expecting a complete run, and spent **2,770,335 tokens** across the three to discover that this model expands to
   fill whatever ceiling it is given on this diff. The right move was one run to read the mechanism (`classification:
   budget`, checked before each round) and then a decision, not two more purchases. The finding is now in the Makefile
   comment so the next person does not repeat it.
6. **I did it again.** A second probe run inside `execute_code` timed out at 280 s and again left an `ocr` child calling
   the model. Found with `pgrep -f opencodereview`, killed, and the lesson is now in the make target's comment: **reviews
   go through `terminal(background=true)` or `make ai-review`, never through a cell with a five-minute cap.**

## Limits, named

- **It cannot measure.** Nothing in a review run executes the project's tests or probes, so every performance claim in a
  note is still ours, and a finding like "this looks quadratic" is a hypothesis.
- **It sees one diff, not the design.** On the test above it did not know D208 was mid-implementation and reported the two
  tests written ahead of the code as failing tests — correct reading of the code, wrong reading of the project's method.
  That is exactly the "fold in, then decide" rule in `AGENTS.md`: those two findings were refused, with the note saying why.
- **The model is a rented opinion.** `kimi-k2.7-code` on the opencode-go endpoint is what is configured today; switching is
  `ocr config set model <id>` plus `ocr llm test`. Any cost figure here belongs to that provider's allowance, not to a
  price list this project controls.
