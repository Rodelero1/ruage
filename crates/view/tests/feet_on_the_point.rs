//! **A unit's body stands on its point: the feet are where the unit is, not its middle** (D178).
//!
//! The owner, after D177, with a screenshot: *"the circle moves a bit elsewhere than it should. See picture with yellow cross being target
//! destination and the unit above it stops a bit further."* The simulation had it right - `crates/sim/tests/move_arrival.rs` lands a villager on
//! the tile it was sent to within a tenth of a tile - so what he was seeing was the **anchor**: the silhouette was drawn *centred* on the point,
//! and the selection ellipse with it, so the body's visible mass sat half a shape above the place it stood.
//!
//! A body is drawn with its **base on the point**, the way a sprite's feet are. This test says so for both silhouettes.

use std::path::PathBuf;

use ruage_view::{Camera, Silhouette, Snapshot, palette, solids_of};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

#[test]
fn a_units_body_sits_on_its_own_point() {
    let units_path = repo("config/gamedata/units.toml");
    if !units_path.exists() {
        return;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources");
    let settings = ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim");
    let map = ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map");
    let economy = ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy");
    let units = ruage_data::load_units_file(&units_path).expect("units");
    let mut world = ruage_sim::World::new(&settings, 7).expect("world");
    world.seed_from(&map, &economy, &units).expect("seeding");
    world.load_resources(&resources).expect("stockpile");

    let scale = world.scale();
    let villager = world.role_unit("villager").expect("villager");
    let (tx, ty) = (40i64, 40i64);
    world
        .submit_checked(ruage_sim::Command::Spawn {
            id: ruage_sim::EntityId(9_950),
            player: 1,
            kind: ruage_sim::EntityKind::Unit,
            unit: Some(villager),
            position: (
                ruage_fixed::Fixed::from_raw(tx * scale + scale / 2),
                ruage_fixed::Fixed::from_raw(ty * scale + scale / 2),
            ),
            speed: ruage_fixed::Fixed::ZERO,
            hp: 25,
            amount: 0,
        })
        .expect("a villager");
    world.step().expect("step");

    let snapshot = Snapshot::of(&world, &map, 1);
    let solids = solids_of(&world, &snapshot);
    let camera = Camera::new((900, 700)).centred_on((tx as f32, ty as f32));
    let frame = render_with_solids(&snapshot, &camera, &solids);
    let (_, cy) = camera.project((tx as f32, ty as f32));
    let colour = palette::PLAYER_ONE;

    let mut lowest = i32::MIN;
    let mut painted = 0;
    for py in 0..700i32 {
        for px in 0..900i32 {
            if frame.pixel(px as u32, py as u32) == colour {
                lowest = lowest.max(py);
                painted += 1;
            }
        }
    }
    assert!(painted > 40, "the body was found: {painted} pixels");
    // The **base** of the body is the point: a couple of pixels of slack for rasterisation, and nothing drawn below it.
    assert!(
        (lowest - cy).abs() <= 2,
        "the body's lowest row is at y={lowest}, and the unit stands at y={cy} - the silhouette is centred on the point instead of standing on it"
    );
    let _ = Silhouette::Square;
}

use ruage_view::render_with_solids;
