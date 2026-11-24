//! **Placement**: where a body may be put, as opposed to where it may walk (D279 keeps these two apart).

use ruage_fixed::Fixed;

use crate::{Entity, EntityKind, World};

impl World {
    /// **The nearest point a body fits**, when a point sits inside a building's footprint grown by its radius (D262-D264).
    /// The game's distance to a footprint is `max(0, |dx| - radius_x)` per axis (`func_00269190`), so touching is legal and
    /// inside is not; a gather aim or a route endpoint that is inside can never be reached by a legal step.
    pub(crate) fn push_out_of_buildings(&self, at: (Fixed, Fixed), radius: i64) -> (Fixed, Fixed) {
        let scale = self.scale;
        let mut point = at;
        for building in self
            .entities
            .iter()
            .filter(|entity| entity.kind == EntityKind::Building)
        {
            let Some((ax, ay, w, h)) = self.footprint_rect(building.id) else {
                continue;
            };
            let left = i64::from(ax) * scale - radius;
            let top = i64::from(ay) * scale - radius;
            let right = (i64::from(ax) + i64::from(w)) * scale + radius;
            let bottom = (i64::from(ay) + i64::from(h)) * scale + radius;
            let (x, y) = (point.0.raw(), point.1.raw());
            if x <= left || x >= right || y <= top || y >= bottom {
                continue;
            }
            let (to_left, to_right) = (x - left, right - x);
            let (to_top, to_bottom) = (y - top, bottom - y);
            let nearest = to_left.min(to_right).min(to_top).min(to_bottom);
            point = if nearest == to_left {
                (Fixed::from_raw(left), point.1)
            } else if nearest == to_right {
                (Fixed::from_raw(right), point.1)
            } else if nearest == to_top {
                (point.0, Fixed::from_raw(top))
            } else {
                (point.0, Fixed::from_raw(bottom))
            };
        }
        point
    }

    /// **Where a body goes when the spot it was given is already taken** (D267) - the original's `func_002699A0`
    /// (`unit_action_resolve.cpp`, 80 lines). Its rule, one for one:
    ///
    /// * the step out is **`ext_a + ext_b + 0.01`** - the two radii plus one hundredth of a tile - so the pair **touches and
    ///   does not overlap**; the 0.01 is on the outside, which is why the original's units stand *just* clear of each other;
    /// * the side is one of **four** (`coord +/- step` on an axis), picked at random per attempt, with the other coordinate
    ///   jittered by a random offset across that side, so a group on one point fans out instead of queueing;
    /// * **fifty** attempts, and the first spot that is on the map, on ground the body fits on, and clear of every other body
    ///   is taken;
    /// * nothing moves an existing body: the newcomer is the one that steps aside (the caller writes this position), which is
    ///   why this is *placement*, not pushing, and D52's "no pushing" still holds.
    ///
    /// The original draws its side and offset from `func_001AAB30` (a random). This one is a xorshift seeded from the entity id
    /// and the tick: the same shape, identical on every machine and every run.
    pub(crate) fn place_apart(&self, at: (Fixed, Fixed), radius: i64, salt: u64) -> (Fixed, Fixed) {
        let reach_to = |other: &Entity, mine: i64| -> i64 {
            match mine + self.body_radius(other.unit) {
                0 => self.scale,
                sum => sum,
            }
        };
        let clear = |point: (Fixed, Fixed), mine: i64| -> bool {
            self.entities.iter().all(|other| {
                if other.kind != EntityKind::Unit || other.id.0 == salt as u32 {
                    return true;
                }
                let reach = reach_to(other, mine);
                let (dx, dy) = (
                    other.position.0.raw() - point.0.raw(),
                    other.position.1.raw() - point.1.raw(),
                );
                dx * dx + dy * dy >= reach * reach
            })
        };
        if clear(at, radius) {
            return at;
        }
        let scale = self.scale;
        let mut state = salt | 1;
        let mut next = move || -> i64 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            i64::try_from(state & 0xFFFF).unwrap_or(0)
        };
        for _ in 0..50 {
            let side = next() % 4;
            // The body that is already standing there decides how far along the side the newcomer may go.
            let span = self
                .entities
                .iter()
                .filter(|other| other.kind == EntityKind::Unit && other.id.0 != salt as u32)
                .map(|other| self.body_radius(other.unit).max(scale / 8))
                .max()
                .unwrap_or(scale / 2);
            let offset = next() % (2 * span) - span;
            // **The step out, on the pair's own measure**: the two radii plus a hundredth of a tile.
            let out = self
                .entities
                .iter()
                .filter(|other| other.kind == EntityKind::Unit && other.id.0 != salt as u32)
                .map(|other| match radius + self.body_radius(other.unit) {
                    0 => scale,
                    sum => sum,
                })
                .max()
                .unwrap_or(scale);
            let step = out + scale / 100;
            let (x, y) = (at.0.raw(), at.1.raw());
            let point = match side {
                0 => (Fixed::from_raw(x + step), Fixed::from_raw(y + offset)),
                1 => (Fixed::from_raw(x - step), Fixed::from_raw(y + offset)),
                2 => (Fixed::from_raw(x + offset), Fixed::from_raw(y + step)),
                _ => (Fixed::from_raw(x + offset), Fixed::from_raw(y - step)),
            };
            let (tx, ty) = self.tile_of_raw(point);
            let (Ok(tx), Ok(ty)) = (u16::try_from(tx), u16::try_from(ty)) else {
                continue;
            };
            if !self.tile_walkable(tx, ty) {
                continue;
            }
            // A body of this radius has to fit on the ground it lands on, and it has to be clear of every other body.
            if self.clearance_at(point, radius) >= 0 && clear(point, radius) {
                return point;
            }
        }
        at
    }
}
