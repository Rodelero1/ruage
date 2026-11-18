# Review: a probe that records which kind of route a refused step belongs to

*Change: one env-gated `eprintln!` in `collide`'s ground branch (`DBG_ORIGIN`). It prints whether the refused route's first point
is a **tile-chain** corner (aligned to a tile point) or a **geometry** corner (offset by the radius), plus the route length. It is
committed rather than left in the working tree on purpose: two earlier readings were void because a `git checkout` in the same
command discarded an uncommitted probe.*

## Performance

The probe is off unless `DBG_ORIGIN` is set, and when on it costs one route lookup and one `eprintln!` per refused step: on the
animals and gather suites the instrumented run took **25 s** against **24 s** uninstrumented, i.e. inside the noise of a 0.6 s
spread. No behaviour is touched - the branch it sits in already returned `WaitGround`.

## Stability

**What it is for.** The register's step 1 needs the *route origin* of a refusal, not its task: the task axis was unanimous (29 of
29 `Gather`) and could not separate a planning defect from a seating one. This probe is the missing axis.

**What was attacked.** Its "tile-chain vs geometry" test is a heuristic: a geometry corner that happens to land on a tile point or
centre would be misread. Grown-rectangle corners sit at `tile edge +- radius`, so they are misclassified only when the radius is
zero or exactly half a tile - and a zero radius *is* a place route, which is precisely what the probe is there to catch. The
misread is therefore informative rather than silent.

Known and accepted: this is an instrument, not a fix. It has no fixture and is not meant to have one; it is removed when the
question it answers is settled.
