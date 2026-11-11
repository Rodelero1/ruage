# Joining a foundation — and the ground as the builder cap

*The owner's `house.aoe2spgame` carries **four** build orders pointing at the same tile, `(76, 78)`, which is what clicking a
half-built house with a crowd looks like. The engine had no gesture for it: `Command::Build` means **place**, and a second
placement on the same ground is refused by `site_refusal` with `SiteOccupied` — correctly. So joining is its own order now, and
the crowd question is answered by a measurement rather than by a rule.*

## What changed

- **`Command::JoinBuild { villager, site }`** — the gesture that was missing. `check_join` requires the site to be that
  player's own **unfinished** building (a new `SimError::NotAFoundation` names anything else), and `start_join` walks the man
  out and puts him on it with the same route the first builder took.
- **Nothing is paid by a join.** The man who placed the site paid for it; the second man's order costs nothing.
- **`Command::Build` is untouched.** See the stability note below — the first attempt overloaded it and that was wrong.

## The measurement that answered the question

With sixteen builders ordered onto one house (`crates/sim/tests/build_ring.rs`):

**at most 11 builders advance a site in a single tick, on a 2×2 footprint with 12 tiles beside it.**

So the free ground *is* the cap, which is what the owner described: *"there were more villagers than the free space around the
house. So they were walking around fruitlessly."* Eleven rather than twelve because the twelfth tile was not reachable from
where that man stood in this fixture. Nothing had to be capped in the engine — the perimeter already bounds it, because two
bodies cannot hold one tile.

## Performance

**The acceptance match** (seed 7, 36,000 ticks, `config/acceptance.toml`), rebuilt binary on both sides:

| | before | after |
|---|---|---|
| hash | `08a111f13b7c43ec` | **`08a111f13b7c43ec`** |
| entities | 150 | **150** |
| repaths | 352 | **352** |
| made_room | 226 | **226** |

**Identical**, which is the right answer for this change: a new command nobody in that scenario issues cannot move the match. It
is also the check that the join did not disturb placement, since every placement in the scenario ran unchanged.

**Wall clock**: 36,000 ticks of that match in **13.20 s** (release, `date +%s%N` around the binary), against the 10.4-11.0 s a
quiet box gives this run - the box was busy with a lint pass at the time, so the figure is quoted as "same match, same work",
not as a regression. The identity of the hash is the measurement; the clock is context.

**The suite** goes from 170 to **172** test binaries (two new files), all green in a full workspace run.

## Stability

- **The first attempt overloaded `Command::Build` and broke two placement rules** — `a_second_site_that_overlaps_the_first_names_its_builder` and `my_own_foundation_is_in_the_way` both went red, exactly as they should: with the overload, ordering a
  building onto ground that already held one **joined** it instead of refusing the placement. That is why the gesture is a
  separate command. Both tests are green again, unchanged, and they are the guard that placement kept its meaning.
- **A join cannot take another player's site, a finished building, or a unit**: `check_join` asks owner, kind and
  `construction.is_some()`, and answers `NotAFoundation` otherwise. The new error has a Display line like every other.
- **A join is idempotent in the harmless direction**: joining the site you are already building simply re-walks you there.
- **The queue is unaffected**: `start_build`'s "already building? then queue it" branch still runs for *placements*; a join never
  enters the queue, so a man cannot be left holding a queued order he did not give.
- **Not covered**: the engine's own AI and script never issue `JoinBuild` yet — they place one site per builder. Wiring the
  script's build step to join an existing site is a separate change, and the acceptance hash above is what proves that nothing
  yet depends on it.
