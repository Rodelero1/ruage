# Review: the names were already there, and the gate had no tests

*The owner, asked what to call the unnamed units: "1 A ... IIRC language DLL files should contain proper names. That way it was
possible to translate the game in hex editor. Age2HD could do it differently though. If none is found there or in other files,
follow A" - and of the two numbers I picked: "2 can you find in original files? It must be somewhere there. Otherwise OK.
Compact, proceed, review."*

## Finding 1: the names were already found, and already in use

`Age2HD/resources/en/strings/key-value/key-value-strings-utf8.txt` (12,593 lines) carries its provenance in its own header:

```
//  ***   Strings from Game/en/language.dll   ***
```

- `5121 "Villager"`, `5149 "Farm"`, `5409 "Halberdier"`, `5661 "Hussar"`, `5123 "Farmer"`.
- Every unit in the dat carries **`name_str_id`** (83 -> 5121, 50 -> 5149, 359 -> 5409, 4 -> 5083).
- `tools/names.py` (D97) has been joining those two all along and writing `config/names.toml`: **1,390 keys for 1,389 units,
  zero duplicates, zero gaps.**

So option **A is unnecessary** and there is nothing for the owner to decide.

## Finding 2: my own inventory measured the wrong column

D119's plan opens with "**56 units have no name, 28 names are duplicated**". Both numbers were true of the **dat's five-letter
codes** in `units.toml` (`VMBAS`, `BARRICADE` x4) and false of the vocabulary: `names.toml` names everything uniquely. The plan's
phase 2 was already done before the plan was written, and I reported a blocker that did not exist. Corrected in
`docs/unit-vocabulary.md`, in D123, and in the tool itself: `--root` and a reader that opens `names.toml`.

## Finding 3: the gate had no tests, and its cycle detector was wrong

`tools/vocabulary.py` decides whether a pack can go name-keyed, and nothing tested it. Six tests now build tiny packs in temp
directories, and one of them found a real defect immediately: the cycle walk was bounded by `len(units)` where `units` is the
**parsed file's top-level key count** (1), so a two-unit cycle - the smallest there is, and the shape the real pack has - was
never reported. Fixed to bound by the unit table. The real pack still reports its own cycle (`RMSHP` 527 and `SDGAL` 1104, the
two fishing ships), and the tests prove a duplicate key, a missing key, a role naming a missing unit and a cycle all block.

## Finding 4: the husk is in the dat, and it is a unit

The farm (50, `FARM`) carries **`dead_unit_id = 357`**: unit 357, `FARM_`, string **5407**, hit points 0,
**`resource_decay_rate = 1.0`**. That is the "effect only" husk the owner described, so it does not have to be invented:

- the faithful model is to **replace the farm with unit 357** when it empties and let its own decay remove it;
- what its `1.0` decay decays is not in the dat, so **the 30-second life stays ours** and is named as ours in D123.

## Finding 5: "nearby" has no constant in the original files

The AI constants carry `sn-lumber-camp-max-distance`, `sn-mining-camp-max-distance` (both commented out) and
`sn-camp-max-distance`. **Nothing for farms.** `free_farm_radius = 12` stays a number of ours, declared as one.

## Performance

No engine code changed in this step, so nothing to re-measure: the tick and snapshot numbers from the adversarial review of
2026-11-05 (**step 6.2 us, snapshot 95.3 us**) stand. The tool itself runs in **562 ms** against the real 1.2 MB pack (measured, not
estimated), so it can gate a build rather than be run by hand; the six fixture tests take **1.44 s** together, mostly
interpreter start-up.

## Stability

| attack | result |
|---|---|
| a unit with no key | blocks, exit 1 (`test_a_unit_with_no_key_blocks_the_pack`) |
| two units sharing a key - the shape "BARRICADE" x4 has | blocks, exit 1 |
| a role naming a unit the table lacks | blocks, exit 1 |
| a two-unit upgrade cycle | blocks, exit 1 - **and this test is what found the bound bug** |
| a unit whose dat code is unreadable but whose key is fine | passes, which is the mistake the first inventory made |
| the real pack | still reports its own cycle and its 2 real forks; "blocked" is the honest verdict for a name-keyed schema until the engine handles multi-parents |

---

*`tools/review_check.py` looks for `## Performance` and `## Stability`; both are above.*
