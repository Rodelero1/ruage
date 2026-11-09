# The decision index: what it costs, and what it got wrong first

*2026-11-09. D163. The tool is `tools/decisions.py`, its tests are `tools/test_decisions.py`, the page it writes is
`docs/site/index.html`.*

## What was built

One page, generated, committed: every decision in `docs/decisions.md`, the basis it names, the questions the U-register
still has open, the paragraphs where a decision names its own limits, and every id cited in the docs or the code that no
decision defines. The owner's ask was "see how they were made ... spot gaps and bad decisions", so the design question was
never the rendering: it was whether a label can be trusted. Every label therefore carries the sentence it was read from,
and the page prints its own coverage - 166 decisions, 0 unplaced headings.

## RED, and what it actually proved

31 tests were written before any production code, and the module was stubbed so every test failed on its own behaviour
rather than on an import (the project's compile-RED is weaker evidence than that).

    Ran 31 tests ... FAILED (failures=11, errors=17)

**Three of those 31 passed at RED, and one of them proved nothing**: `test_a_table_row_does_not_become_a_heading_decision`
asserts an empty corpus has no unclassified headings, which is true of a parser that returns nothing. It is a control, not
a test, and it is recorded here rather than quietly counted.

Eleven more tests were written during the work, after the behaviour existed. Those cannot be RED, so each was proved by
mutation instead, and the mutation families that matter are:

| mutation | test that fails |
|---|---|
| `_esc` returns the text unchanged | `...markup_is_escaped_not_executed` |
| headings read at `##` only | `...written_one_level_down_is_still_a_decision` |
| the gap scan keeps heading paragraphs | `...prose_subheading_does_not_cut_its_decision_in_short` |
| origin tags lose their `tag-` prefix | `...prefixed_so_they_cannot_be_counted_as_cards` |
| correction edges never recorded | `...corrects_an_earlier_one_marks_the_earlier_as_corrected` |

## My own errors, in the order they happened

1. **Four of my first tests were wrong, not the code.** A "version string" fixture (`D-v0.58 and D16's rule`) contained a
   real reference to D16; `any(not u.closed for u in unknowns)` iterated the dict's keys; a coverage assertion expected
   the prose `0 unclassified` where the page emits `<b>0</b> unclassified`; and a test demanded that a heading with no
   decision id be "unclassified" when document structure is exactly what it is.
2. **One test was loose where it mattered and tight where it did not.** "The page has no outside dependency" asserted
   `https://` appears nowhere - but the page quotes evidence, and evidence quotes wiki URLs. It now checks that nothing is
   *fetched* (`src="http`, `href="http`, `<link rel=`).
3. **`corrected by DD2`** - I formatted an id that already had its prefix.
4. **The origin tag class collided with the card class.** An origin called `decision` produced `<span class="tag decision">`,
   which the filter script counted as a card: the page said **"180 of 164 shown"** in a real browser. Found by looking,
   not by a test; the tags are `tag-<origin>` now and a test pins the prefix.
5. **The filter's first version did nothing.** `[].some.call(set, fn)` reads `length`, and a `Set` has `size`, so every
   chip returned **0 of 164 shown**. Found in the browser; nothing in the Python suite could have seen it.
6. **Ids were matched with three digits at most.** a heading with a four-digit id failed the `\b` and was silently filed as *document
   structure* - the quiet failure the whole design is against. Found by a 6,560-decision scaling probe, never by the
   corpus, which is at D163.
7. **A heading that says "not modelled" became a gap.** The new `###` test caught it: the gap list contained the heading
   and its paragraph, so one named limit counted twice.
8. **The light-mode CSS had a trailing comma** (`.panel,.decision,{...}`), so it applied to nothing, and what it did set
   was the body colour only - cards stayed dark on a light page. Both modes now override the custom properties.
9. **`python3 tools/decisions.py | head` printed a traceback** - `BrokenPipeError`, hit live while piping the summary.
10. **The snippet scan was quadratic in section length.** One 300 KB decision took **6.4 s**; a 1 MB one **67 s**. The
    cause was `rfind` from the start of the section for every match, and computing a snippet for matches that were then
    thrown away. Windowed and capped: **0.33 s** and **1.1 s** (19x and 60x).
11. **The corpus's own vocabulary poisoned the classifier.** D163 lists the tag names, so the decision explaining the
    labels was tagged with **eight** of them. A sentence naming three or more origins is now treated as a taxonomy, and
    categories named inside a quotation attributed to the owner are his words, not our method (that one came from the
    owner's own phrase "reverse engineered").
12. **The id D162 was taken while I worked.** Another session in this tree appended *D162 - the virtual fence* and its own
    `v1.130` row to the same files, uncommitted. My decision was renumbered to **D163 / v1.131**, and the doc commits are
    built from the *committed* corpus plus my own hunks only - another writer's prose is not mine to commit (AGENTS.md).
    The page found this too: two `## D162` headings is exactly the shape of a duplicate it exists to surface.
13. **The compact was written after the code.** Step 1 of the routine says persist the state *first*; `docs/next-step.md`,
    `docs/decisions.md` and the SPEC row were written once the tool worked. Nothing was lost, but the order was inverted
    and a session resuming mid-step would have had no note.

## Performance

Measured on this box, corpus 312 KB / 4,000 lines, 166 decisions. Median of 5 unless stated.

- **`make decisions`, end to end: 1.24 s** (min 1.23, max 1.37). Of that, `parse_decisions` **424 ms**, `scan_sources`
  **25 ms** (356 files, 5.5 MB), `dangling_refs` **148 ms**, `render` **1.5 ms**, plus interpreter start and the write.
- **Page: 197 KB** for 166 decisions = **1.19 KB per decision**, in **one file**, with **zero** external requests - checked
  in a browser, and by a test that no `src=`/`href=` names a remote host.
- **A single long section: 300 KB → 326 ms** (was 6,437 ms), **1 MB → 1,113 ms** (was 67,142 ms). This is the one
  measurement that changed a design rather than confirming one.
- **Scaling: linear after the fix.** 164 → 26 ms, 1,640 → 301 ms, 3,280 → 487 ms, 6,560 → 924 ms.
- **10,000 decisions: parse 0.81 s, render 0.06 s, page 4.6 MB.** The page is what fails first, not the parser.
- **The tick loop pays nothing.** The tool is not linked into any crate; `cargo test` and the acceptance hash are
  untouched. `tools-test` is 151 tests in 9.7 s, of which the 42 decision tests are 1.7 s - and most of that is the
  deliberately oversized section in the cost test.

Where the 424 ms goes: 65 compiled patterns swept over 312 KB, roughly **20 MB of regex input**. Precompiling cut the
cache lookups but not the scan (measured: unchanged at 325 ms). The lever, if the corpus ever gets ten times larger, is
to run each pattern only over the sections that contain its literal keyword.

## Stability

Each of these was run, not reasoned about:

| attack | result |
|---|---|
| a heading the parser cannot place (`## D2 has no separator`) | exit **1**, `UNPLACED: ## D2 has no separator` on stderr |
| the corpus missing | exit **2**, `cannot read the corpus: ... is not there`, no traceback |
| a corpus with CRLF line endings | parses; 1 decision, 1 gap |
| a title with `<script>alert("x")</script>` and a gap with `<img onerror>` | escaped; a test asserts both |
| an empty corpus | renders; "0 decisions", and the empty-list copy |
| a title with emoji and accents | renders, escaped |
| a four-digit id | parses as a decision, no longer filed as structure |
| output piped into `head` | exit 0, no traceback |
| the page reloaded while it is being written | cannot see a half file: the build writes `.tmp` and `os.replace`s it |

**What is not covered, said plainly.** The atomic write has **no test** - proving it needs a fault injected between write
and rename, and a test that only checks "no `.tmp` was left behind" would pass without the rename as well. The filter
script has **no test at all**: both of its bugs (the `Set`/`some` one and the class collision) were found by driving the
page in a browser, which is not repeatable in `make check`. And the classifier's accuracy has **no test worth the name** -
`test_every_decision_without_a_basis_is_listed_so_it_can_be_triaged` only asserts that *not all* decisions are uncited,
which is a weak assertion guarding a strong claim. What makes the claim checkable is the design, not this suite: every
label is printed next to the sentence it came from.

## What the page found on its first run

- **10 ids cited and never written**: D78, D79, D81, D82, D115, D122b, D124, D125b, D126, D136. D78-D82 are a hole in
  the numbering that four documents still reference.
- **33 of 166 decisions name no source at all.**
- **24 named gaps** - "not modelled" paragraphs that no list anywhere else collects.
- **12 open questions** in the U-register, and **22 decisions** carrying unresolved language.
- Three decisions live at `###` under their parent (D24a, D94a, D98a) and were invisible to a reading that looks for `##`.
- Two `## D162` headings, while the other session's work was uncommitted - the duplicate this page exists to surface.
  (That section is not in this commit; the page was built from the *committed* corpus, so D162 shows as a broken
  reference until the other session lands and the page is rebuilt.)

## Not done

- **No decision-to-test map.** 155 of 178 `.rs` files cite a decision id, but nothing checks that the cited decision
  exists in the test's behaviour or that the test still exists. Inferring it from comments would be a guess.
- **One U-register only.** A question written outside `docs/mechanics/README.md` is not in the open register.
- **`tools/` is excluded from the reference scan**, because its fixtures name ids that are meant not to exist. That is a
  choice, not an oversight: including it would drown the signal in 20 fabricated ids.
- **No hook.** `make decisions` is manual and the page can go stale between commits; wiring it into `make check` would make
  every commit pay 1.2 s for a docs artefact that cannot regress the engine.
