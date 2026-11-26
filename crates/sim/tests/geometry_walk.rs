//! Ray hit and edge-walk core: first grown-rect hit along a ray, and the
//! closed ring of the merged shape the hit rect belongs to.

use ruage_sim::{Clear, ray_hit, verify_path, walk_shape};

fn two_boxes_on_x_axis() -> Vec<Clear> {
    vec![
        Clear {
            at: (30_000, 0),
            half: (5_000, 5_000),
            reach: 0,
        },
        Clear {
            at: (70_000, 0),
            half: (5_000, 5_000),
            reach: 0,
        },
    ]
}

fn aligned_overlapping_pair() -> Vec<Clear> {
    vec![
        Clear {
            at: (0, 0),
            half: (10_000, 10_000),
            reach: 0,
        },
        Clear {
            at: (10_000, 0),
            half: (10_000, 10_000),
            reach: 0,
        },
    ]
}

#[test]
fn ray_hits_first_rect_only() {
    let hit = ray_hit((0, 0), (100_000, 0), &two_boxes_on_x_axis());
    assert_eq!(hit, Some(0));
    assert_eq!(ray_hit((0, 0), (10_000, 0), &two_boxes_on_x_axis()), None);
}

#[test]
fn aligned_overlap_bridges_instead_of_wrong_turn() {
    let ring = walk_shape(0, &aligned_overlapping_pair());
    assert!(verify_path(&ring, &aligned_overlapping_pair()).is_none());
    assert!(ring.len() >= 4);
}
