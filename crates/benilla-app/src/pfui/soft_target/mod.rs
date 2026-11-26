//! ClassicAPI's soft targeting (`target/SoftUnit.cpp`, `SoftInteract.cpp`, `Priority.cpp`): the
//! soft enemy, the soft friend and the soft interact object, picked from the priority candidate
//! list ([`priority`]) and fed to the VM's tokens and events
//! ([`benilla_ui::script::pfui_host::soft_targets`]).
//!
//! Every 100 ms (`TargetPriorityUpdateDelay`) the list is rebuilt from the units and GameObjects
//! around the player: a unit within bank 1's far range plus both reaches (16 yd dead), a
//! GameObject within 16 yd; each classified enemy (`can_attack`), friend (friendly either way,
//! selectable, assistable) and interact (the cursor over it would offer an interaction, the
//! world classifier's own verdict), the player's channel object (the fishing bobber) its own
//! object. The picks follow ClassicAPI: each side on when its CVar is 2 or 3 and its range above
//! 0; with `SoftTargetMatchLocked` the target stands in for its side; each frame a pick that left
//! its class or its range is dropped. The `any…` units are ClassicAPI's `GetAny`.
// Deviation: 1.12 has no loose targets, so every target counts as one the player chose (locked);
// the model box is the bounding radius and the box height (`OverheadFallback`) times the scale,
// 1 x 2 yd for a GameObject, where ClassicAPI reads the model's bounds; the boss, critter, party
// and PvP classes are not built; the icons, circles, names, tooltip and nameplate dim, and the
// soft cast and soft attack, are not drawn or built.

pub(crate) mod priority;

use std::time::{Duration, Instant};

use benilla_protocol::messages::ObjectType;
use benilla_ui::script::pfui_host::soft_targets::{self, SoftPicks};
use benilla_ui::script::UiScript;
use bevy::prelude::*;

use crate::target::cursor_mode::{InteractReach, Reach};
use priority::{flag, Camera, Candidate, List, View, BANK};

/// `TargetPriorityUpdateDelay`.
const UPDATE_DELAY: Duration = Duration::from_millis(100);
const UNIT_FLAG_NOT_SELECTABLE: u32 = 1 << 25;
const UNIT_FLAG_IN_COMBAT: u32 = 0x0008_0000;
const UNIT_FLAG_PET_IN_COMBAT: u32 = 0x0000_0800;

/// The feed's set, for a reader of the same poses to order behind.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SoftTargetFeed;

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        feed.in_set(super::InputSnapshots)
            .in_set(SoftTargetFeed)
            .run_if(crate::ui_script::ingame_ui_up),
    );
}

/// The CVars the picks read, each frame.
struct Settings {
    enemy: bool,
    enemy_arc: i32,
    enemy_range: f32,
    friend: bool,
    friend_arc: i32,
    friend_range: f32,
    interact: bool,
    interact_arc: i32,
    interact_range: f32,
    interact_range_hard: bool,
    match_locked: i32,
    with_locked: i32,
    relaxation: bool,
    combat_lock: i32,
}

impl Settings {
    /// Whether any soft side is switched on: the three CVars alone, which is all an off frame
    /// reads.
    fn enabled(script: &UiScript) -> bool {
        ["SoftTargetEnemy", "SoftTargetFriend", "SoftTargetInteract"]
            .iter()
            .any(|name| {
                script
                    .cvar(name)
                    .and_then(|v| v.trim().parse::<f32>().ok())
                    .is_some_and(|v| matches!(v as i32, 2 | 3))
            })
    }

    fn read(script: &UiScript) -> Self {
        let get = |name: &str| -> f32 {
            script
                .cvar(name)
                .and_then(|v| v.trim().parse::<f32>().ok())
                .unwrap_or(0.0)
        };
        let on = |name: &str| matches!(get(name) as i32, 2 | 3);
        Self {
            enemy: on("SoftTargetEnemy"),
            enemy_arc: get("SoftTargetEnemyArc") as i32,
            enemy_range: get("SoftTargetEnemyRange"),
            friend: on("SoftTargetFriend"),
            friend_arc: get("SoftTargetFriendArc") as i32,
            friend_range: get("SoftTargetFriendRange"),
            interact: on("SoftTargetInteract"),
            interact_arc: get("SoftTargetInteractArc") as i32,
            interact_range: get("SoftTargetInteractRange"),
            interact_range_hard: get("SoftTargetInteractRangeIsHard") != 0.0,
            match_locked: get("SoftTargetMatchLocked") as i32,
            with_locked: get("SoftTargetWithLocked") as i32,
            relaxation: get("TargetPriorityCombatLockContextualRelaxation") != 0.0,
            combat_lock: get("TargetPriorityCombatLock") as i32,
        }
    }
}

#[derive(Default)]
struct State {
    /// The picks are on the VM (soft targeting was on last frame).
    active: bool,
    built: Option<Instant>,
    list: List,
    enemy: Option<u64>,
    friend: Option<u64>,
    interact: Option<u64>,
    /// The soft interact object is usable now (in reach), not only out of reach.
    interact_now: bool,
}

fn wow(v: Vec3) -> [f32; 3] {
    benilla_assets::coords::bevy_to_wow(v)
}

/// A direction in WoW's frame.
fn wow_dir(at: Vec3, dir: Vec3) -> [f32; 3] {
    let (a, b) = (wow(at), wow(at + dir));
    [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
}

fn dist_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)] // a Bevy system's inputs
fn feed(
    script: Option<NonSend<UiScript>>,
    mut state: Local<State>,
    player: Option<Res<crate::player::Player>>,
    selection: Res<crate::target::Selection>,
    me: Query<
        (
            Entity,
            &crate::net::Guid,
            &crate::net::ObjectStore,
            &Transform,
        ),
        With<crate::net::SelfPlayer>,
    >,
    objects: Query<(
        Entity,
        &crate::net::Guid,
        &crate::net::ObjectStore,
        &Transform,
        Option<&crate::entities::OverheadFallback>,
    )>,
    camera: Query<
        (&GlobalTransform, &bevy::camera::Camera),
        With<benilla_world::view::WorldCamera>,
    >,
    relations: crate::target::RelationWorld,
    reach: InteractReach,
    collision: benilla_world::collision::WorldCollision,
) {
    let Some(script) = script else { return };
    let me = me.single().ok();
    // Off (the default): three CVar reads a frame, and the picks cleared once, so the `any…`
    // tokens read the live target as with no feed.
    if !Settings::enabled(&script) || me.is_none() {
        if state.active {
            *state = State::default();
            soft_targets::clear(script.lua());
        }
        return;
    }
    let settings = Settings::read(&script);
    state.active = true;
    let target = selection.guid;
    let now = Instant::now();
    let (me_entity, me_guid, me_store, me_place) = me.expect("checked");
    let me_pos = wow(me_place.translation);
    let reach_of = |guid: u64| -> Reach {
        objects.iter().find(|(_, g, ..)| g.0 == guid).map_or(
            Reach::None,
            |(entity, _, store, ..)| {
                reach.of(
                    entity,
                    guid,
                    store.0.object_type() == Some(ObjectType::GameObject),
                )
            },
        )
    };
    let pos_of = |guid: u64| {
        objects
            .iter()
            .find(|(_, g, ..)| g.0 == guid)
            .map(|(_, _, _, place, _)| wow(place.translation))
    };
    // ── The rebuild, every 100 ms ──
    let due = state
        .built
        .is_none_or(|at| now.duration_since(at) >= UPDATE_DELAY);
    if due {
        state.built = Some(now);
        let Ok((camera_place, camera)) = camera.single() else {
            return;
        };
        let rel = relations.get();
        let me_rel = relations.unit_of(me_guid.0, me_store);
        let my_reach = me_store.0.unit_bounding_radius();
        let me_in_combat = me_store.0.unit_flags() & UNIT_FLAG_PET_IN_COMBAT != 0
            || me_store.0.unit_target().is_some_and(|t| t != 0)
                && me_store.0.unit_flags() & UNIT_FLAG_IN_COMBAT != 0;
        let channel = me_store.0.unit_channel_object().unwrap_or(0);
        let mut entries = Vec::new();
        for (entity, guid, store, place, fallback) in &objects {
            if entity == me_entity || guid.0 == me_guid.0 {
                continue;
            }
            let pos = wow(place.translation);
            let scale = store.0.object_scale_x().unwrap_or(1.0).max(0.0);
            let mut e = Candidate {
                guid: guid.0,
                pos,
                ..Default::default()
            };
            match store.0.object_type() {
                Some(ObjectType::GameObject) => {
                    if dist_sq(pos, me_pos) > BANK.interact_range.powi(2) {
                        continue;
                    }
                    e.flags = flag::GAME_OBJECT;
                    if reach.of(entity, guid.0, true) == Reach::None {
                        continue;
                    }
                    e.flags |= flag::INTERACT;
                    if guid.0 == channel {
                        e.flags |= flag::OWN_OBJECT;
                    }
                    (e.radius, e.height) = (1.0, 2.0);
                }
                Some(ObjectType::Unit | ObjectType::Player) => {
                    let it_reach = store.0.unit_bounding_radius();
                    let dead = store.0.unit_health() == Some(0);
                    e.flags = flag::UNIT;
                    let range = if dead {
                        e.flags |= flag::DEAD;
                        BANK.interact_range
                    } else {
                        BANK.range_far + my_reach + it_reach
                    };
                    if dist_sq(pos, me_pos) > range * range {
                        continue;
                    }
                    if store.0.object_type() == Some(ObjectType::Player) {
                        e.flags |= flag::PLAYER;
                    }
                    let unit = relations.unit_of(guid.0, store);
                    let unit_flags = store.0.unit_flags();
                    if !dead {
                        if rel.can_attack(&me_rel, &unit) {
                            e.flags |= flag::ENEMY;
                            let engaged = unit_flags & UNIT_FLAG_PET_IN_COMBAT != 0
                                || store.0.unit_target().is_some_and(|t| t != 0);
                            let in_combat = unit_flags & UNIT_FLAG_IN_COMBAT != 0;
                            let locked = if settings.combat_lock == 1 {
                                engaged || in_combat
                            } else {
                                engaged && in_combat
                            };
                            if me_in_combat && locked {
                                e.flags |= flag::COMBAT_LOCK;
                            }
                        }
                        let friendly =
                            rel.reaction(&me_rel, &unit) >= 4 || rel.reaction(&unit, &me_rel) >= 4;
                        if friendly
                            && unit_flags & UNIT_FLAG_NOT_SELECTABLE == 0
                            && rel.can_assist(&me_rel, &unit)
                        {
                            e.flags |= flag::FRIEND;
                        }
                    }
                    if reach.of(entity, guid.0, false) != Reach::None {
                        e.flags |= flag::INTERACT;
                    }
                    if e.flags & (flag::ENEMY | flag::FRIEND | flag::INTERACT) == 0 {
                        continue;
                    }
                    e.reach = it_reach;
                    e.radius = it_reach;
                    e.height = fallback.map_or(2.0, |f| f.0) * scale;
                    let melee = (it_reach + my_reach + 1.333_33).max(5.0);
                    e.hits_me = store.0.unit_target() == Some(me_guid.0)
                        && dist_sq(pos, me_pos) < melee * melee;
                }
                _ => continue,
            }
            entries.push(e);
        }
        let forward = camera_place.forward().as_vec3();
        let left = camera_place.left().as_vec3();
        let camera_pos = camera_place.translation();
        let cam = Camera {
            pos: wow(camera_pos),
            forward: wow_dir(camera_pos, forward),
            left: wow_dir(camera_pos, left),
            yaw: 0.0,
        };
        let cam = Camera {
            yaw: cam.forward[1].atan2(cam.forward[0]),
            ..cam
        };
        let facing_dir = player.as_ref().map_or(Vec3::NEG_Z, |p| {
            Quat::from_rotation_y(p.facing()) * Vec3::NEG_Z
        });
        let f = wow_dir(me_place.translation, facing_dir);
        let in_frustum = |e: &Candidate| {
            let mut outside = 0x1f_u32;
            for i in 0..8 {
                let corner = [
                    e.pos[0] + if i & 1 != 0 { e.radius } else { -e.radius },
                    e.pos[1] + if i & 2 != 0 { e.radius } else { -e.radius },
                    e.pos[2] + e.base + if i & 4 != 0 { e.height } else { 0.0 },
                ];
                let point = benilla_assets::coords::wow_to_bevy(corner);
                let out = match camera.world_to_ndc(camera_place, point) {
                    Some(ndc) if ndc.z >= 0.0 => {
                        let k = 0.8;
                        (u32::from(ndc.x < -k))
                            | (u32::from(ndc.x > k) << 1)
                            | (u32::from(ndc.y < -k) << 2)
                            | (u32::from(ndc.y > k) << 3)
                    }
                    _ => 0x10,
                };
                outside &= out;
                if outside == 0 {
                    return true;
                }
            }
            false
        };
        let clear = |a: [f32; 3], b: [f32; 3]| {
            collision.clear_line(
                benilla_assets::coords::wow_to_bevy(a),
                benilla_assets::coords::wow_to_bevy(b),
            )
        };
        let view = View {
            player: me_pos,
            player_base: 0.0,
            player_height: 2.0,
            player_reach: my_reach,
            facing: f[1].atan2(f[0]),
            camera: cam,
            in_frustum: &in_frustum,
            clear: &clear,
        };
        let target_pos = target.and_then(pos_of);
        state.list = List::build(&view, target_pos, settings.relaxation, entries);
    }
    // ── The picks (SoftUnit::OnFrameTick) ──
    let allowed = target.is_none() || settings.with_locked != 0;
    if !allowed {
        state.enemy = None;
        state.friend = None;
        state.interact = None;
    } else if due {
        let locked = match target {
            Some(t) if matches!(settings.match_locked, 1 | 2) => state.list.flags_of(t),
            _ => 0,
        };
        let pick = |on: bool, range: f32, kind: u32, arc: i32| {
            if locked & kind != 0 {
                return target;
            }
            if !on || range <= 0.0 {
                return None;
            }
            state.list.first(kind, arc)
        };
        let enemy = pick(
            settings.enemy,
            settings.enemy_range,
            flag::ENEMY,
            settings.enemy_arc,
        );
        let friend = pick(
            settings.friend,
            settings.friend_range,
            flag::FRIEND,
            settings.friend_arc,
        );
        let interact_on =
            settings.interact && (settings.interact_range > 0.0 || !settings.interact_range_hard);
        let interact = if locked & flag::INTERACT != 0 && interact_on {
            match target.filter(|t| reach_of(*t) == Reach::Now) {
                Some(t) => Some(t),
                None => state
                    .list
                    .first(flag::INTERACT | flag::OWN_OBJECT, 2)
                    .or_else(|| state.list.first(flag::INTERACT, settings.interact_arc)),
            }
        } else if interact_on {
            state
                .list
                .first(flag::INTERACT | flag::OWN_OBJECT, 2)
                .or_else(|| state.list.first(flag::INTERACT, settings.interact_arc))
        } else {
            None
        };
        (state.enemy, state.friend, state.interact) = (enemy, friend, interact);
    }
    // Validate: a pick that left its class or its range is dropped, every frame.
    let within =
        |guid: u64, range: f32| pos_of(guid).is_some_and(|p| dist_sq(p, me_pos) <= range * range);
    let keep_side = |guid: Option<u64>, kind: u32, range: f32| {
        guid.filter(|g| {
            state.list.flags_of(*g) & (kind | flag::CRITTER) == kind && within(*g, range)
        })
    };
    let mut enemy = keep_side(state.enemy, flag::ENEMY, settings.enemy_range);
    if enemy.is_some() && enemy == target && settings.with_locked != 2 {
        let attacking = me_store.0.unit_target().is_some_and(|t| t != 0)
            && me_store.0.unit_flags() & UNIT_FLAG_IN_COMBAT != 0;
        if attacking {
            enemy = None;
        }
    }
    let friend = keep_side(state.friend, flag::FRIEND, settings.friend_range);
    let mut interact_now = false;
    let interact = state.interact.filter(|g| {
        let r = reach_of(*g);
        if state.list.flags_of(*g) & flag::INTERACT == 0 || r == Reach::None {
            return false;
        }
        interact_now = r == Reach::Now;
        within(*g, settings.interact_range) || (!settings.interact_range_hard && interact_now)
    });
    (
        state.enemy,
        state.friend,
        state.interact,
        state.interact_now,
    ) = (enemy, friend, interact, interact_now);
    // `GetAny`: with no soft unit the target; the soft unit while there is no target or it is the
    // target; the target when it is a candidate of the side; else the soft unit.
    let any = |soft: Option<u64>, kind: u32| match (soft, target) {
        (None, t) => t,
        (Some(s), None) => Some(s),
        (Some(s), Some(t)) if s == t => Some(s),
        (Some(s), Some(t)) => {
            if state.list.flags_of(t) & kind == kind {
                Some(t)
            } else {
                Some(s)
            }
        }
    };
    let any_interact = match (interact, target) {
        (None, t) => t,
        (Some(s), Some(t)) if s != t && state.list.flags_of(t) & flag::INTERACT != 0 => {
            if reach_of(t) == Reach::Now {
                Some(t)
            } else {
                Some(s)
            }
        }
        (Some(s), _) => Some(s),
    };
    soft_targets::set(
        script.lua(),
        SoftPicks {
            enemy,
            friend,
            interact,
            any_enemy: any(enemy, flag::ENEMY),
            any_friend: any(friend, flag::FRIEND),
            any_interact,
        },
    );
}
