# OPEN — next tranche after ADR 0020 (owner picks)

**Where this stands:** ADRs 0001–0020 signed; index at 300 rows; `origin/adr-migration` at `4088a78` (0020 draft; sign-off commit lands this step). This compact is the agent's handoff.

**Prior implementation (ADR 0020, signed, this step):**
- Take-gate: `at_node = box_gap_sq <= step^2` in the gather arm; dead arrival predecessors deleted (`touching`/`touch_tolerance`, `own_place`, `ring_open`-as-arrival, `beside_adjacent`, settle scaffolding, `let _` sink) plus orphan cascade (`ring_open`, `ring_point`, `GATHER_RING_MILLI`, `GATHER_PLACES`, lib `SIN` — tiles keeps its own copy).
- Pin: `crates/sim/tests/take_gate.rs` 3/3 (within-one-step takes, two-steps-out walks, absence guard watched failing first).
- Live rules: `box_gap_sq`, `node_radius`, `solid_boxes`, `target_contact`, target-named orders, grown boxes in search + query, overlap (`<`) collision kept as named deliberate difference.
- Measures today: full suite 942 passed / 3 failed / 23 ignored (reds: 2 known `modifiers` Town-Watch + 1 foreign scratch); `take_gate` 3/3, `gather_touching` 3/3, acceptance 2 passed + 1 ignored; clippy `-D warnings` clean (lib + pin targets), fmt clean.
- Setup correction: this box CAN run the suite — `CARGO_HOME=/tmp/home/.cargo` (env default `/usr/local/cargo` is empty). Prior "impossible on this box" notes withdrawn as wrong-path, numbers stand.

**Adopted and binding:** adopt-from-game (0019) + conditional grow+test+reset (0018) + one box ruler (0020) — box-vs-grown-box, flag+margin, terrain point-tested; past stands for every body; forty-tick same-kind re-target, displacement gone; working margins 0.95 gather / 1.0 walk; no second distance anywhere.

**Candidates for the next tranche (owner picks one):**
1. D183–D185 — foundations; D175–D179, D187, D193–D197 — area leftovers.
2. D242–D245 — R1/invariants follow-ups to 0019 (R1a/R1b split, four-body invariants).
3. D287/D288 — relay cut + gather re-target rule (sibling thread of the take-gate batch).

**To do, in order:**
1. Owner names the tranche.
2. Draft the ADR (retirement record), plain language, test-first if code is touched.
3. Owner verdict: sign / amend / rework (one word signs).
4. On sign: delete bank rows; MANIFEST per-D-id rows → new ADR; `make decision-index`; regen live files; review note with Performance + Stability; commit; push.

No SPEC file in this tree (cited by section only); no changelog file (review notes + bank rows are the changelog).
