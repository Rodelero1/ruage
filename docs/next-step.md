# OPEN - the repath rise is answered; next is ErrorClass and the small tools

**State, 2026-11-12 late.** D192 landed. Its +70% repath rise is **answered**: bodies stopped forcing re-plans altogether
(**31 -> 0**) and every added re-plan is **ground** (122 -> 166 on the minimal scenario, 166 over 36,000 ticks). Instruments:
`repaths_by_body()`/`repaths_by_ground()`, printed by the headless as `repaths_body=`/`repaths_ground=`; test
`repath_reasons::a_body_on_the_line_is_counted_as_a_body`.

**Done from that queue:** `ErrorClass` (D194), the `next-step.md` cap (D195, 350,038 -> 1,230 bytes), the SPEC §11 test rule and five
stale docstrings (D193), `StepDecision` + `step_trace` (D196, which found enemy units being overlapped at rest: 0.237 against 0.4), and
repaths by reason (D197).

**Also done:** the inline-decision parser (four decisions the index could not see: 198 -> 202 parsed, dangling 12 -> 7) and
`docs/dangling.toml`, a ratchet that refuses a *new* dangling id.

**Also done:** `crates/script` + `crates/harness` (D198) - the scenario language and the runners left the engine, which found three
real layering breaks (three `World` queries the engine called from `script.rs`, two private methods the runner needed, and a binary test
that could only live with its binary). Acceptance hash unchanged at `91572e72c33cee93`; 183 binaries.

**The queue, in order, no decisions needed:** (1) `Vision` sub-struct (the god-object split, smallest slice first).

**Needs the owner:** `crates/save` + `--load` (licensing, `docs/licensing.md`), and anything that changes a rule the owner set.

**After that**, the architecture review's ranked backlog: `Step` enum + `step_trace`, the `next-step.md` cap, the test-invariant rule,
`ErrorClass`, `crates/script`, `crates/save`.
