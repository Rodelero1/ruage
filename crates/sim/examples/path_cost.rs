//! **What a path search costs**, so a change to the search can be priced instead of argued (D143).
//!
//! Run it with:
//!
//! ```sh
//! cargo run --release --offline -p ruage-sim --example path_cost
//! ```
//!
//! It builds the `Walk` the acceptance map gives, picks a long pair of tiles, and times `find_path` over and over,
//! printing the mean and the checksum of every route. The checksum is the point: a search that got faster by finding
//! different routes has not been optimised, it has been broken, and the acceptance hash cannot see a change made only on
//! the error path.

use std::path::PathBuf;
use std::time::Instant;

use ruage_sim::{Walk, find_path};

const SEARCHES: usize = 2_000;

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn main() {
    let config = repo("config");
    let resources = match ruage_data::load_resources_file(&config.join("resources.toml")) {
        Ok(pack) => pack,
        Err(err) => {
            eprintln!("no resources.toml ({err}): nothing to measure");
            return;
        }
    };
    let map = match ruage_data::load_map_file(&config.join("map.toml"), &resources) {
        Ok(map) => map,
        Err(err) => {
            eprintln!("no map.toml ({err}): nothing to measure");
            return;
        }
    };
    let walk = Walk::from_map(&map);
    let (width, height) = walk.size();

    // The longest free pair the map allows: two corners, so the search expands the whole grid rather than two tiles.
    let from = (1u16, 1u16);
    let to = (width - 2, height - 2);
    if walk.blocked(from.0, from.1) || walk.blocked(to.0, to.1) {
        eprintln!("the corners are not free on this map: pick another pair");
        return;
    }

    // **Three lengths, because a flat array is not free on a short search.** Two `Vec`s the size of the map are one
    // allocation each, and the initialisation is O(cells): that is nothing against a corner-to-corner search and could be
    // everything against a three-tile one. A change priced only on the long case would be priced wrong.
    let pairs = [
        ("corner to corner", from, to),
        ("mid, ~10 tiles", (16u16, 16u16), (26u16, 26u16)),
        ("short, ~3 tiles", (30u16, 30u16), (32u16, 31u16)),
    ];
    for (label, from, to) in pairs {
        if walk.blocked(from.0, from.1) || walk.blocked(to.0, to.1) {
            println!("{label}: endpoints are blocked on this map, skipped");
            continue;
        }
        let length = find_path(&walk, from, to).map_or(0, |path| path.len());
        let start = Instant::now();
        let mut checksum = 0u64;
        for _ in 0..SEARCHES {
            if let Some(path) = find_path(&walk, from, to) {
                // Order-sensitive: a different route gives a different sum, which is how "faster" and "different" are told
                // apart.
                checksum = checksum.wrapping_add(path.len() as u64);
                for (index, tile) in path.iter().enumerate() {
                    checksum = checksum.wrapping_mul(31).wrapping_add(
                        (u64::from(tile.0) << 8) ^ (u64::from(tile.1) << 1) ^ index as u64,
                    );
                }
            }
        }
        let elapsed = start.elapsed();
        let per_search = elapsed.as_secs_f64() * 1e6 / SEARCHES as f64;
        println!(
            "map {width}x{height}, {label} ({from:?} to {to:?}): {length} waypoints, {SEARCHES} searches in {:.1} ms, \
             {per_search:.1} us/search, checksum {checksum:016x}",
            elapsed.as_secs_f64() * 1e3
        );
    }
}
