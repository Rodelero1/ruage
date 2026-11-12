
### D189 - your own units pass through each other on the way

**The owner chose it**, between the two ways out of the crowd problem: *"1"* - bodies may overlap while walking, collision at rest
only. His words for the need were *"I think that capping is pointless"* and the rule behind it, *"either villager fits in or
don't"*.

**The defect it fixes, measured.** With bodies solid, ten men sent at one bush cannot be delivered: the men behind cannot pass the
men in front, and the settle - which moves a body **directly**, not through the collision pass - either stepped inside a neighbour
(closest pair **0.144**, then **0.345**) or left a man stranded outside the band (**1.543**, **1.962** tiles from the bush, never
working). Whichever step direction was tried, one of the two failed.

**The rule.** A unit with **more than one waypoint left** ignores other **friendly** bodies: it takes the step it planned, and two
of the player's own villagers may overlap in transit. **The last step still respects a body**, because the final approach is where
order matters: without that bound a builder stacked on an idle villager and never reached its footprint (a house at **hp 1** after
2,000 ticks), and arrivals became a race rather than a queue. **Enemy bodies are unchanged**: `bound_to_reach` still holds two
players' units apart, which is the pass D131 was about.

At rest, the gather ring keeps men apart by reservation rather than by collision: a place is a point on the ring one body (0.40)
clear of every other place, and the settle walks a man exactly onto his place (`off_place` is any deviation, not a tolerance - a
0.125 tolerance let two men finish **0.376** apart, and 0.025 let them finish 0.345).

**Measured on landing:** ten men work one bush (nine was the ceiling), closest pair of places **0.408**; eighteen work one tree at a
body each, which is `ring_capacity()`; the acceptance hash and counters are in the review.
