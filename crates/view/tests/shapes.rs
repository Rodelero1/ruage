//! Silhouettes, pictograms and the speed selector: the three things the owner asked for so that a glance at the picture
//! says what is happening. Written before the drawing code (D7).

use std::path::PathBuf;

use ruage_data::{Economy, Map, Units};
use ruage_fixed::Fixed;
use ruage_sim::{Command, EntityId, EntityKind, World};
use ruage_view::{Camera, Mark, Pace, Silhouette, Snapshot, Speed, render};

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    settings: ruage_data::Sim,
    map: Map,
    economy: Economy,
    resources: ruage_data::Resources,
    units: Units,
    combat: ruage_data::Combat,
}

fn fixture() -> Option<Fixture> {
    let units = repo("config/gamedata/units.toml");
    if !units.exists() {
        return None;
    }
    let resources =
        ruage_data::load_resources_file(&repo("config/resources.toml")).expect("resources.toml");
    Some(Fixture {
        settings: ruage_data::load_sim_file(&repo("config/sim.toml")).expect("sim.toml"),
        map: ruage_data::load_map_file(&repo("config/map.toml"), &resources).expect("map.toml"),
        economy: ruage_data::load_economy_file(&repo("config/economy.toml")).expect("economy.toml"),
        resources,
        units: ruage_data::load_units_file(&units).expect("units.toml"),
        combat: ruage_data::load_combat_file(&repo("config/combat.toml")).expect("combat.toml"),
    })
}

fn seeded_with(fx: &Fixture, economy: &Economy) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

fn seeded(fx: &Fixture) -> World {
    let mut world = World::new(&fx.settings, 7).expect("world");
    world
        .seed_from(&fx.map, &fx.economy, &fx.units)
        .expect("seeding");
    world
        .load_resources(&fx.resources)
        .expect("the pack fits the stockpile");
    world.load_combat(&fx.combat);
    world
}

/// Place one unit on open ground at a tile and return its id.
fn place(world: &mut World, player: u8, unit: u32, tile: (i64, i64)) -> EntityId {
    let scale = world.scale();
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    world
        .submit_checked(Command::Spawn {
            id,
            player,
            kind: EntityKind::Unit,
            unit: Some(unit),
            position: (
                Fixed::from_raw(tile.0 * scale),
                Fixed::from_raw(tile.1 * scale),
            ),
            speed: Fixed::ZERO,
            hp: 40,
            amount: 0,
        })
        .expect("a unit");
    world.step().expect("one tick");
    id
}

/// The pixels of one **exact colour** in a box around a pixel: what a shape of that colour actually covers. Comparing
/// against a "ground" pixel is not enough - grass tones differ per tile and the map's own colours are not the unit's.
fn painted(
    frame: &ruage_view::Frame,
    centre: (i32, i32),
    radius: i32,
    colour: [u8; 4],
) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for y in (centre.1 - radius)..=(centre.1 + radius) {
        for x in (centre.0 - radius)..=(centre.0 + radius) {
            if frame.pixel(x as u32, y as u32) == colour {
                out.push((x, y));
            }
        }
    }
    out
}

/// **A villager is a square and a soldier is a triangle, from content's own roles** (item 6 of the silhouettes). The same
/// unit at the same pixel must paint a different shape, and the difference must be the shape rather than the colour:
/// the soldier's row widths grow towards its base, the villager's are constant.
#[test]
fn a_villager_and_a_soldier_at_one_pixel_differ() {
    let Some(fx) = fixture() else { return };
    let villager = fx.economy.role["villager"];
    let soldier = fx.economy.role["militia"];

    let shapes: Vec<(Silhouette, Vec<i32>)> = [villager, soldier]
        .into_iter()
        .map(|unit| {
            let mut world = seeded(&fx);
            let id = place(&mut world, 1, unit, (30, 30));
            let snapshot = Snapshot::of(&world, &fx.map, 1);
            let camera = Camera::new((320, 240)).centred_on((30.0, 30.0));
            // **The body's own centre, not the tile point** (D168/D178): a unit placed at an integer position stands at the corner
            // of four diamonds, and since D178 its body rises from that point rather than straddling it, so a window centred on the
            // tile point missed it entirely and read forty zero rows.
            let centre = camera.project(ruage_view::drawn_at(
                snapshot
                    .entities
                    .iter()
                    .find(|entity| entity.id == id.0)
                    .expect("the placed unit")
                    .position,
            ));
            let frame = render(&snapshot, &camera);
            // **The unit this test placed**, not the first unit in the snapshot: the opening seeds villagers, and asking
            // for "a unit" found one of those - both shapes came back Square and the test "passed" the wrong question.
            let entity = snapshot
                .entities
                .iter()
                .find(|entity| entity.id == id.0)
                .expect("the placed unit");
            // Rows of the unit's own colour, clamped to the frame: `Frame::pixel` indexes raw, so a negative
            // coordinate is not "outside", it is a wrapped index that reads somebody else's pixel.
            let (w, h) = (frame.width as i32, frame.height as i32);
            let mut widths = Vec::new();
            for y in (centre.1 - 20)..=(centre.1 + 20) {
                if y < 0 || y >= h {
                    widths.push(0);
                    continue;
                }
                let painted = (-28..=28)
                    .filter(|dx| {
                        let x = centre.0 + dx;
                        x >= 0
                            && x < w
                            && frame.pixel(x as u32, y as u32) == ruage_view::palette::PLAYER_ONE
                    })
                    .count() as i32;
                widths.push(painted);
            }
            (entity.shape, widths)
        })
        .collect();

    assert_eq!(shapes[0].0, Silhouette::Square, "the villager is a square");
    assert_eq!(
        shapes[1].0,
        Silhouette::Triangle,
        "the militia is a triangle"
    );
    assert_ne!(
        shapes[0].1, shapes[1].1,
        "a villager and a soldier paint different rows: {:?} against {:?}",
        shapes[0].1, shapes[1].1
    );
    // A square is the same width on every row it covers; a triangle grows towards its base.
    let square: Vec<i32> = shapes[0].1.iter().copied().filter(|w| *w > 0).collect();
    let triangle: Vec<i32> = shapes[1].1.iter().copied().filter(|w| *w > 0).collect();
    assert!(
        square.windows(2).all(|pair| pair[0] == pair[1]),
        "a square's rows are constant: {square:?}"
    );
    // **A triangle's rows narrow towards its apex**, which now sits at the top of a body anchored by its base (D178); before the
    // anchor change the scan window was wider than the body, so the rows only ever grew towards the base and the assertion was written
    // the other way round. Both directions are "not a square"; the strict one is the shape's own.
    assert!(
        triangle.windows(2).all(|pair| pair[1] <= pair[0])
            && triangle.windows(2).any(|pair| pair[1] < pair[0]),
        "a triangle's rows narrow towards its apex: {triangle:?}"
    );
}

/// **A building carries the mark content names**, and a role content names no mark for is drawn plain (item 6).
#[test]
fn a_building_carries_the_mark_its_role_names() {
    let Some(fx) = fixture() else { return };
    // Two content packs that differ in one line: a house with a mark and a house without. The project pack itself gives
    // every building a mark, so the "plain" world has to be built rather than assumed.
    let mut marked_economy = fx.economy.clone();
    marked_economy
        .mark
        .insert("house".to_owned(), "roof".to_owned());
    let mut plain_economy = fx.economy.clone();
    plain_economy.mark.remove("house");

    let house = fx.economy.role["house"];
    let mut plain = seeded_with(&fx, &plain_economy);
    let scale = plain.scale();
    let id = EntityId(
        plain
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    plain
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(30 * scale), Fixed::from_raw(30 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    plain.step().expect("one tick");

    let mut marked = seeded_with(&fx, &marked_economy);
    marked
        .submit_checked(Command::Spawn {
            id,
            player: 1,
            kind: EntityKind::Building,
            unit: Some(house),
            position: (Fixed::from_raw(30 * scale), Fixed::from_raw(30 * scale)),
            speed: Fixed::ZERO,
            hp: 550,
            amount: 0,
        })
        .expect("a house");
    marked.step().expect("one tick");

    let plain_snapshot = Snapshot::of(&plain, &fx.map, 1);
    let marked_snapshot = Snapshot::of(&marked, &fx.map, 1);
    let plain_house = plain_snapshot
        .entities
        .iter()
        .find(|entity| entity.id == id.0)
        .expect("the plain house");
    let marked_house = marked_snapshot
        .entities
        .iter()
        .find(|entity| entity.id == id.0)
        .expect("the marked house");
    assert_eq!(plain_house.mark, Mark::None, "content names no mark");
    assert_eq!(
        marked_house.mark,
        Mark::Roof,
        "content says the house has a roof"
    );

    let camera = Camera::new((320, 240)).centred_on((30.0, 30.0));
    let plain_frame = render(&plain_snapshot, &camera);
    let marked_frame = render(&marked_snapshot, &camera);
    let centre = camera.project((30.0, 30.0));
    assert_ne!(
        painted(&plain_frame, centre, 20, ruage_view::palette::MARK),
        painted(&marked_frame, centre, 20, ruage_view::palette::MARK),
        "the mark is not on the footprint"
    );
}

/// **The speed selector is pacing, not state** (the owner's speed item): 1.5x must be exact over time and a change
/// mid-match must neither give nor take a partial tick.
#[test]
fn the_speeds_are_exact_and_a_change_mid_match_keeps_the_carry() {
    let mut pace = Pace::new(Speed::One);
    let mut ticks = 0;
    for _ in 0..200 {
        ticks += pace.ticks();
    }
    assert_eq!(ticks, 200, "1x runs one tick an interval");

    pace.set(Speed::OneAndAHalf);
    let mut ticks = 0;
    for _ in 0..200 {
        ticks += pace.ticks();
    }
    assert_eq!(ticks, 300, "1.5x is exact over two hundred intervals");

    for (speed, expected) in [
        (Speed::Two, 400u32),
        (Speed::Three, 600),
        (Speed::Five, 1000),
    ] {
        let mut pace = Pace::new(speed);
        let mut ticks = 0;
        for _ in 0..200 {
            ticks += pace.ticks();
        }
        assert_eq!(ticks, expected, "{} over 200 intervals", speed.label());
    }

    // A change mid-match keeps whatever fraction is in the accumulator: one interval of 1x carries nothing, and the four
    // intervals after switching to 1.5x run 1, 2, 1, 2 - six ticks over four intervals, which is 1.5x exactly, with no
    // interval ever rounding the half tick away.
    let mut pace = Pace::new(Speed::One);
    assert_eq!(pace.ticks(), 1);
    pace.set(Speed::OneAndAHalf);
    assert_eq!(
        vec![pace.ticks(), pace.ticks(), pace.ticks(), pace.ticks()],
        vec![1, 2, 1, 2]
    );
}

/// **The speed selector is pickable** (the owner's speed item): five boxes that do not overlap, each answering with its
/// own speed, nothing outside, and the live one drawn filled.
#[test]
fn the_speed_selector_picks_one_of_five() {
    // **The frame's own size**, not a nominal one: the selector is laid out for the frame it is drawn in, and a window
    // that is not 720 tall puts the same boxes somewhere else - which is a click that misses, once.
    let (width, height) = (960u32, 720u32);
    let boxes = ruage_view::ui::speed_boxes(width, height);
    for (index, &(x, y, w, h)) in boxes.iter().enumerate() {
        let centre = (x + w as i32 / 2, y + h as i32 / 2);
        assert_eq!(
            ruage_view::ui::speed_at(centre, width, height),
            Some(Speed::ALL[index]),
            "box {index} at {centre:?} picks its own speed"
        );
        // The boxes are a row that does not overlap: every other box's centre is outside this one.
        for (other, &(ox, oy, ow, oh)) in boxes.iter().enumerate() {
            if other == index {
                continue;
            }
            let outside = ox + ow as i32 <= x
                || ox >= x + w as i32
                || oy + oh as i32 <= y
                || oy >= y + h as i32;
            assert!(outside, "boxes {index} and {other} overlap");
        }
    }
    assert_eq!(
        ruage_view::ui::speed_at((5, 5), width, height),
        None,
        "the left of the bar is the stockpile, not the selector"
    );

    // The live speed's box is drawn filled; the others are outlined.
    let mut one = ruage_view::Frame::new(width, height);
    let mut five = ruage_view::Frame::new(width, height);
    ruage_view::ui::speed_selector(&mut one, Speed::One);
    ruage_view::ui::speed_selector(&mut five, Speed::Five);
    let box_of = |frame: &ruage_view::Frame, index: usize| {
        let (x, y, w, h) = ruage_view::ui::speed_boxes(width, height)[index];
        frame.pixel((x + w as i32 / 2) as u32, (y + h as i32 / 2) as u32)
    };
    assert_ne!(
        box_of(&one, 0),
        box_of(&one, 1),
        "the live box is filled and its neighbour is not"
    );
    assert_eq!(
        box_of(&one, 0),
        ruage_view::palette::SELECTION,
        "the filled box is the selection colour"
    );
    assert_eq!(box_of(&five, 4), ruage_view::palette::SELECTION);
}
