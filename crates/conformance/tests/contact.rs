//! **RULE: a body touches an object when its box reaches the object's box - edges included.**
//!
//! Source: `FUN_00840830` (the REA corpus), whose own comparison is
//! `query_x1 <= obj_x2 && obj_x1 <= query_x2 && query_y1 <= obj_y2 && obj_y1 <= query_y2` - four `<=`, so **touching is a
//! collision** - and it grows the query box by the mover's radius (`in_ECX + 0x268`) before asking. This file pins the one
//! thing that is easy to get subtly wrong: a body standing exactly at its own radius from a wall is *against* it, not clear of
//! it, and the engine must say so.
//!
//! Its partner rule is ours by necessity, not by choice: a body the map *places* inside a box (a fixture spawn, a body left
//! touching) may walk **out** and along, and may not go deeper. The original never has that case - its placement phase
//! (`func_002699A0`) keeps bodies out of objects - so the escape rule is the grid's answer to a state the game cannot reach.

use ruage_conformance::{RADIUS, SCALE, centre, ground};

/// RULE: a point exactly at the grown face touches the object.
#[test]
fn a_point_exactly_at_the_grown_face_touches() {
    // One blocked tile at (6,6) on an empty field: it spans [120000, 140000] on each axis at this scale, so its centre is
    // (130000, 130000) and its box grown by a 0.2-radius body reaches 14,000 raw from that centre.
    let field = ground(16, 16, &[(6, 6)]);
    let (scale, reach) = (SCALE, RADIUS);
    let centre = 6 * scale + scale / 2;
    let face = centre - (scale / 2 + reach);

    // Exactly on the grown face: touching, so a collision.
    assert!(
        field.touches_object(scale, (face, centre), reach),
        "a body exactly at its radius from the face counts as touching it"
    );
    // One raw unit further out: clear.
    assert!(
        !field.touches_object(scale, (face - 1, centre), reach),
        "one raw unit outside the face is clear of the box"
    );
    // A corner is touching too - the test is per axis, not a distance.
    assert!(
        field.touches_object(scale, (face, face), reach),
        "a body at the box's corner reaches both faces"
    );
    assert!(
        !field.touches_object(scale, (face - 1, face - 1), reach),
        "one raw unit past the corner is clear"
    );
}

/// RULE: the three-by-three neighbourhood answers for every body the dat carries.
///
/// The box is grown by the mover's radius, so a body at the widest radius in the pack (a trebuchet, half a tile) reaches
/// exactly one tile from the object's centre - and no further. A wider body would need a bigger scan, which is why the
/// engine asserts the bound rather than silently answering for a neighbourhood that no longer covers the answer.
#[test]
fn the_widest_body_in_the_pack_is_the_edge_of_the_scan() {
    let field = ground(16, 16, &[(6, 6)]);
    let widest = SCALE / 2;
    // One tile away from the blocked tile's centre: touching at exactly half a tile of radius.
    let beside = (6 * SCALE + SCALE / 2 + SCALE, 6 * SCALE + SCALE / 2);
    assert!(
        field.touches_object(SCALE, beside, widest),
        "a half-tile body one tile out is exactly touching"
    );
    let beyond = (6 * SCALE + SCALE / 2 + SCALE + 1, 6 * SCALE + SCALE / 2);
    assert!(
        !field.touches_object(SCALE, beyond, widest),
        "one raw unit beyond contact is clear"
    );
    let _ = centre(6, 6);
}
