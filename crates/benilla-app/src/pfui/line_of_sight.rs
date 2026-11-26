//! The host half of ClassicAPI's `UnitInLineOfSight`
//! ([`benilla_ui::script::pfui_host::line_of_sight`]): every 100 ms, one segment from the player
//! to each unit the VM asked about lately (none asked, no ray), false past 150 yards (ClassicAPI's
//! guard), each end raised by its own unit's collision
//! height ([`CollisionHeight`], the reference's `CMovement+0xb4`, as ClassicAPI reads it),
//! traced through the world's line-of-sight geometry ([`WorldCollision::clear_line`]).
//!
//! [`WorldCollision::clear_line`]: benilla_world::collision::WorldCollision::clear_line
//! [`CollisionHeight`]: crate::entities::CollisionHeight

use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use benilla_protocol::messages::ObjectType;
use benilla_ui::script::pfui_host::line_of_sight;
use benilla_ui::script::UiScript;
use bevy::prelude::*;

const UPDATE_DELAY: Duration = Duration::from_millis(100);
/// ClassicAPI's `kMaxTraceYards`: past it a unit reads as blocked.
const MAX_TRACE: f32 = 150.0;

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        // Behind the soft-target feed, which reads the same poses and collision after the movers.
        trace
            .in_set(super::InputSnapshots)
            .after(super::soft_target::SoftTargetFeed)
            .run_if(crate::ui_script::ingame_ui_up),
    );
}

#[allow(clippy::type_complexity)] // a Bevy system's inputs
fn trace(
    script: Option<NonSend<UiScript>>,
    mut last: Local<Option<Instant>>,
    me: Query<
        (
            &crate::net::Guid,
            &Transform,
            Option<&crate::entities::CollisionHeight>,
        ),
        With<crate::net::SelfPlayer>,
    >,
    units: Query<(
        &crate::net::Guid,
        &crate::net::ObjectStore,
        &Transform,
        Option<&crate::entities::CollisionHeight>,
    )>,
    collision: benilla_world::collision::WorldCollision,
) {
    let Some(script) = script else { return };
    let now = Instant::now();
    if last.is_some_and(|at| now.duration_since(at) < UPDATE_DELAY) {
        return;
    }
    *last = Some(now);
    // Only the units the VM asked about lately: nothing asked, no ray.
    let wanted: HashSet<u64> = line_of_sight::wanted(script.lua()).into_iter().collect();
    let Some((me_guid, me_place, me_height)) = me.single().ok().filter(|_| !wanted.is_empty())
    else {
        line_of_sight::set(script.lua(), HashMap::new());
        return;
    };
    let height = |h: Option<&crate::entities::CollisionHeight>| h.copied().unwrap_or_default().0;
    let eye = me_place.translation + Vec3::Y * height(me_height);
    let mut clear = HashMap::new();
    for (guid, store, place, collision_height) in &units {
        if !wanted.contains(&guid.0)
            || guid.0 == me_guid.0
            || !matches!(
                store.0.object_type(),
                Some(ObjectType::Unit | ObjectType::Player)
            )
        {
            continue;
        }
        let in_reach = place.translation.distance(me_place.translation) <= MAX_TRACE;
        let to = place.translation + Vec3::Y * height(collision_height);
        clear.insert(guid.0, in_reach && collision.clear_line(eye, to));
    }
    line_of_sight::set(script.lua(), clear);
}
