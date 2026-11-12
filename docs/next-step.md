# OPEN - the repath rise is answered; next is ErrorClass and the small tools

**State, 2026-11-12 late.** D192 landed. Its +70% repath rise is **answered**: bodies stopped forcing re-plans altogether
(**31 -> 0**) and every added re-plan is **ground** (122 -> 166 on the minimal scenario, 166 over 36,000 ticks). Instruments:
`repaths_by_body()`/`repaths_by_ground()`, printed by the headless as `repaths_body=`/`repaths_ground=`; test
`repath_reasons::a_body_on_the_line_is_counted_as_a_body`.

**Done from that queue:** `ErrorClass` (D194), the `next-step.md` cap (D195, 350,038 -> 1,230 bytes), the SPEC §11 test rule and five
stale docstrings (D193), `StepDecision` + `step_trace` (D196, which found enemy units being overlapped at rest: 0.237 against 0.4), and
repaths by reason (D197).

**The queue, in order, no decisions needed:** (1) `crates/script` extraction (1,656 lines of AI and scenario language inside the sim
crate, against D17/D18); (2) resolve the 11 dangling decision ids and the D190 gap; (3) `Vision` sub-struct.

**Needs the owner:** `crates/save` + `--load` (licensing, `docs/licensing.md`), and anything that changes a rule the owner set.

**After that**, the architecture review's ranked backlog: `Step` enum + `step_trace`, the `next-step.md` cap, the test-invariant rule,
`ErrorClass`, `crates/script`, `crates/save`.
