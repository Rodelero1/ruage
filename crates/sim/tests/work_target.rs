//! **What a working villager's target is** (D121): the panel needs the target's role and its resource column so the content
//! can call him a Forager, a Farmer or a Lumberjack.
//!
//! Written against the pack rather than a fixture: the point of the accessor is that it reads the *real* role table and the
//! *real* column of a node, and a made-up world would prove neither.

use ruage_data::Resources;
use ruage_sim::{Command, EntityId, EntityKind, World};

mod common;

fn fresh() -> Option<(World, Resources)> {
    common::fresh_with_resources()
}

fn villager(world: &World) -> EntityId {
    world
        .entities()
        .iter()
        .find(|entity| entity.player == 1 && entity.kind == EntityKind::Unit)
        .map(|entity| entity.id)
        .expect("the seed puts a villager on the map")
}

/// The nearest resource node holding `slot`, so the villager has somewhere to go.
fn node_of(world: &World, slot: usize, from: EntityId) -> Option<EntityId> {
    let (x, y) = world.position(from)?;
    let (x, y) = (x.raw(), y.raw());
    world
        .entities()
        .iter()
        .filter(|entity| matches!(entity.kind, EntityKind::Resource(s) if s == slot))
        .min_by_key(|entity| {
            let dx = entity.position.0.raw() - x;
            let dy = entity.position.1.raw() - y;
            (dx / 1000) * (dx / 1000) + (dy / 1000) * (dy / 1000)
        })
        .map(|entity| entity.id)
}

#[test]
fn a_woodcutter_targets_a_column_and_not_a_role() {
    let Some((mut world, resources)) = fresh() else {
        return;
    };
    let wood = resources
        .slot_of("wood")
        .expect("the pack has a wood column");
    let worker = villager(&world);
    let tree = node_of(&world, wood, worker).expect("the map seeds trees");
    world
        .submit_checked(Command::Gather {
            villager: worker,
            node: tree,
        })
        .expect("a villager can be sent to a tree");
    // One tick is enough: the task is taken when the order lands, and the walk is part of it.
    world.step().expect("stepping");

    let target = world.work_target(worker).expect("he is working on a tree");
    assert_eq!(
        target.column,
        Some(wood),
        "a tree holds wood, and that is what the panel's word hangs on"
    );
    assert_eq!(
        target.role, None,
        "the reference pack gives a tree no role, which is why the column key exists"
    );
}

#[test]
fn an_idle_villager_has_no_target_and_keeps_his_name() {
    let Some((world, _)) = fresh() else {
        return;
    };
    let worker = villager(&world);
    assert!(
        world.work_target(worker).is_none(),
        "an idle villager is a Villager: no target, so the content has no job word to give"
    );
}

#[test]
fn a_gathered_sheep_is_targeted_by_its_role() {
    let Some((mut world, _)) = fresh() else {
        return;
    };
    let sheep_unit = world.role_unit("sheep").expect("a sheep role");
    let worker = villager(&world);
    let (x, y) = world.position(worker).expect("the villager");
    let id = EntityId(
        world
            .entities()
            .iter()
            .map(|entity| entity.id.0)
            .max()
            .unwrap_or(0)
            + 1,
    );
    let scale = world.scale();
    world
        .submit_checked(Command::Spawn {
            id,
            player: 0,
            kind: EntityKind::Unit,
            unit: Some(sheep_unit),
            position: (ruage_fixed::Fixed::from_raw(x.raw() + 3 * scale), y),
            speed: ruage_fixed::Fixed::from_raw(scale / 2),
            hp: 7,
            amount: 0,
        })
        .expect("a sheep on clear ground");
    world.step().expect("one tick");
    world
        .submit_checked(Command::Gather {
            villager: worker,
            node: id,
        })
        .expect("a villager can be sent to a sheep");
    world.step().expect("stepping");

    let target = world.work_target(worker).expect("he is working on a sheep");
    assert_eq!(
        target.role.as_deref(),
        Some("sheep"),
        "a sheep has a role, so the content names the job by that role: Shepherd"
    );
}
