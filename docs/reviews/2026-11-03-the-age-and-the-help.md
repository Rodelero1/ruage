# The bar says what age it is, and the pointer explains itself

**Step:** U3 of the interface work (2026-11-03): the age readout, the game's five top-bar buttons, and the rollover help.
The owner named all three as the point of having a top bar at all.

---

## 1. The age, and where its order comes from

The bar's middle was a 700-pixel hole. It now reads the age in the pack's words - `DARK AGE` at the start of a match,
`FEUDAL AGE` after the advance.

**The order is content's, because neither a map nor an id can give it.** `[age.requires]` is a `BTreeMap` keyed by role, so
iterating gives the alphabetical order (`castle_age`, `feudal_age`, `imperial_age`), and the tech ids do not help either:
dark 104, feudal 101, castle 102, imperial 103. So `config/economy.toml` grew one line:

```toml
order = ["dark_age", "feudal_age", "castle_age", "imperial_age"]
```

`World::age_name(player)` walks that list and answers the last age whose tech is researched - or the first when none is -
and **`None` when a pack gives no order**, because guessing ("the alphabetically last key is the latest age") would put a
wrong word on the screen. A name in `order` that is not in `[tech_role]` is a **load error**, not an age that never arrives.
`Snapshot.age` carries it to the view, which draws it and nothing else.

Test (`tech_tree.rs`): dark at the start, feudal after `feudal_age`, castle after `castle_age`, and a later age does not
replace an earlier one.

## 2. The game's five buttons, and the speed selector's new home

The bar's right end had our speed selector in it, because that is where it was put when there was nothing else to put
there. The game puts five buttons there - **MENU, STATS, CHAT, DIPLO, TECH** - so they are there now, labelled placeholders
(the owner's choice), with `ui::bar_button_at` as their one hit test. **MENU opens the menu; the other four say "X is not
built yet" in the notice line.** A button that opens nothing and says nothing is worse than no button.

The **speed selector moved to the bottom of the selected zone**, right-aligned: pacing is the window's not the match's
(D101), and the game's bar is for the game's buttons. It is the same five-box control, same keys, same hit test.

## 3. The help box

`Layout.help` has been in the layout since U1 and empty since. It now shows one line, in words the interface already has:
the hovered command button's name and its refusal ("SPEARMAN - unit unavailable"), or the bar button's name. The window
fills `Hud.help` from its own hover state, so the box cannot describe something that is not under the pointer.

Two of openaok's ideas in one small box: `SetWorldHint` (whoever knows what the cursor is over sets the line, every frame)
and the rule that the help names the refusal in the interface's own words rather than the engine's sentence.

## Performance

- One `draw_text` at one pixel a dot for the age, five small widgets with a three-to-five-letter label each, and the help
  line only while the pointer is over something. Measured with the benchmark: **117 us** a frame (U2: 116, U1: 90-112,
  pre-layout: 151-195) - the bar's new contents are inside the run-to-run spread.
- The layout grew one array (`speed`); no new allocation per frame.

## Stability

- **No invented words.** The age comes from the pack, the button labels are ours and marked as placeholders, and the help
  line is built from data the UI already holds.
- **The speed control moved rather than duplicated**: one `speed_boxes`, one `speed_at`, one hit test, wherever it is drawn.
- **A bar button that is not wired says so**, in the same notice channel as a refused order, so a player is never left
  guessing whether a click registered.
- **Known gaps:** the help box does not yet describe what a right-click would do in the world (openaok's `SetWorldHint`
  for the map), and STATS/CHAT/DIPLO/TECH are placeholders. Both are recorded rather than dressed up.
