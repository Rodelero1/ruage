# OPEN - the repath rise is answered; next is ErrorClass and the small tools

**State, 2026-11-12 late.** D192 landed. Its +70% repath rise is **answered**: bodies stopped forcing re-plans altogether
(**31 -> 0**) and every added re-plan is **ground** (122 -> 166 on the minimal scenario, 166 over 36,000 ticks). Instruments:
`repaths_by_body()`/`repaths_by_ground()`, printed by the headless as `repaths_body=`/`repaths_ground=`; test
`repath_reasons::a_body_on_the_line_is_counted_as_a_body`.

**Done from that queue:** `ErrorClass` (59 variants, three classes, exhaustive match so a new one cannot compile unclassified) and
the `next-step.md` cap (350,038 -> 1,230 bytes, the rest in `docs/history/`, enforced by `make check` and the pre-commit hook).

**The queue, in order, no decisions needed:** (1) the SPEC §11 rule that tests assert relations and quote measurements, converting
the gather suite; (2) `Step` enum + `step_trace`; (3) `crates/script` extraction; (4) resolve the 12 dangling decision ids and the
D190 gap; (5) `Vision` sub-struct.

**Needs the owner:** `crates/save` + `--load` (licensing, `docs/licensing.md`), and anything that changes a rule the owner set.

**After that**, the architecture review's ranked backlog: `Step` enum + `step_trace`, the `next-step.md` cap, the test-invariant rule,
`ErrorClass`, `crates/script`, `crates/save`.
