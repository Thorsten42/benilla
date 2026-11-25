//! ClassicAPI's encounter (`combatlog/Encounter.cpp`, `unit/Boss.cpp`): the bosses behind
//! `boss1`..`boss5` and `INSTANCE_ENCOUNTER_ENGAGE_UNIT`.
//!
//! 1.12's server sends no engage messages, so a boss is a world boss (creature rank 3) or a
//! creature an addon listed (`C_InstanceEncounter.AddBossCreatureID`). The encounter starts at the
//! first combat log entry between a hostile, living boss and a unit of the group (the player, a
//! party or raid member, a pet of theirs), other than a kill or an aura falling off; later bosses
//! the group fights join it. It ends with a kill once every engaged boss is dead, friendly or (at a
//! death) gone from view; with a wipe once the group was seen fighting and no member in view is in
//! combat with a boss still standing; and as a wipe at a loading screen. Every boss joining and
//! every boss leaving at the end fires `INSTANCE_ENCOUNTER_ENGAGE_UNIT`, with no payload.
// Deviation: ClassicAPI also writes ENCOUNTER_START / ENCOUNTER_END lines to the combat log file,
// which this client does not write.

use mlua::Lua;

use super::Metadata;

/// `UNIT_FLAG_IN_COMBAT`.
const IN_COMBAT: u32 = 0x0008_0000;
/// The combat log's `COMBATLOG_OBJECT_REACTION_HOSTILE` and its group affiliations.
const REACTION_HOSTILE: u32 = 0x40;
const AFFILIATION_GROUP: u32 = 0x1 | 0x2 | 0x4;
const WORLD_BOSS_RANK: u32 = 3;

#[derive(Default)]
struct Encounter {
    active: bool,
    saw_fighting: bool,
    /// The engaged bosses in the order they joined, each with whether it died.
    bosses: Vec<(u64, bool)>,
}

/// A creature's entry from its 1.12 GUID (high word 0xF130), else `None`.
fn creature_id(guid: u64) -> Option<u32> {
    (guid >> 48 == 0xF130).then_some(((guid >> 24) & 0xFF_FFFF) as u32)
}

fn is_boss(metadata: &Metadata, guid: u64) -> bool {
    let Some(id) = creature_id(guid) else {
        return false;
    };
    metadata.boss_creatures.contains(&id)
        || metadata
            .states
            .get(&guid)
            .is_some_and(|s| s.rank == WORLD_BOSS_RANK)
}

fn dead(metadata: &Metadata, guid: u64) -> bool {
    metadata
        .states
        .get(&guid)
        .is_some_and(|s| s.has_object && (s.dead || s.health == 0))
}

/// In view at all: its state carries an object.
fn in_view(metadata: &Metadata, guid: u64) -> bool {
    metadata.states.get(&guid).is_some_and(|s| s.has_object)
}

/// Friendly to the player: its reaction (stored plus one) is friendly or better.
fn friendly(metadata: &Metadata, guid: u64) -> bool {
    metadata.states.get(&guid).is_some_and(|s| s.reaction >= 5)
}

fn group_fighting(lua: &Lua, metadata: &Metadata) -> bool {
    let Some(model) = lua.app_data_ref::<crate::script::Model>() else {
        return false;
    };
    let guids = &model.unit_guids;
    std::iter::once(guids.player)
        .chain(guids.raid.iter().copied())
        .chain(guids.party.iter().copied())
        .filter(|g| *g != 0)
        .any(|g| {
            metadata
                .states
                .get(&g)
                .is_some_and(|s| s.has_object && s.flags & IN_COMBAT != 0)
        })
}

/// Every engaged boss is beaten: dead, friendly, or (with `gone_counts`) out of view.
fn beaten(metadata: &Metadata, state: &Encounter, gone_counts: bool) -> bool {
    state.bosses.iter().all(|&(guid, died)| {
        died || if in_view(metadata, guid) {
            friendly(metadata, guid)
        } else {
            gone_counts
        }
    })
}

/// Push the boss list behind the tokens and fire the engage event once per change.
fn publish(lua: &Lua, changes: usize) {
    let bosses: Vec<u64> = lua
        .app_data_ref::<Encounter>()
        .map(|s| s.bosses.iter().map(|b| b.0).collect())
        .unwrap_or_default();
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        metadata.bosses = bosses;
    }
    for _ in 0..changes {
        super::super::fire_pfui_event(lua, "INSTANCE_ENCOUNTER_ENGAGE_UNIT", vec![]);
    }
}

fn end(lua: &Lua) {
    let gone = lua.app_data_mut::<Encounter>().map_or(0, |mut s| {
        s.active = false;
        std::mem::take(&mut s.bosses).len()
    });
    publish(lua, gone);
}

fn engage(lua: &Lua, guid: u64, flags: u32) {
    if flags & REACTION_HOSTILE == 0 {
        return;
    }
    let boss = lua
        .app_data_ref::<Metadata>()
        .is_some_and(|m| is_boss(&m, guid) && !dead(&m, guid));
    if !boss {
        return;
    }
    let joined = {
        let Some(mut state) = lua.app_data_mut::<Encounter>() else {
            return;
        };
        if !state.active {
            *state = Encounter {
                active: true,
                saw_fighting: false,
                bosses: vec![(guid, false)],
            };
            true
        } else if state.bosses.iter().all(|b| b.0 != guid) {
            state.bosses.push((guid, false));
            true
        } else {
            false
        }
    };
    if joined {
        publish(lua, 1);
    }
}

/// One combat log entry: `subevent`, its source and its target with their flags.
pub fn observe(lua: &Lua, subevent: &str, source: Option<(u64, u32)>, dest: Option<(u64, u32)>) {
    if lua.app_data_ref::<Encounter>().is_none() {
        lua.set_app_data(Encounter::default());
    }
    if matches!(subevent, "UNIT_DIED" | "UNIT_DESTROYED") {
        let Some((guid, _)) = dest else { return };
        let killed = {
            let Some(mut state) = lua.app_data_mut::<Encounter>() else {
                return;
            };
            let Some(boss) = state.bosses.iter_mut().find(|b| b.0 == guid) else {
                return;
            };
            boss.1 = true;
            true
        };
        let over = killed
            && lua.app_data_ref::<Metadata>().is_some_and(|m| {
                lua.app_data_ref::<Encounter>()
                    .is_some_and(|s| s.active && beaten(&m, &s, true))
            });
        if over {
            end(lua);
        }
        return;
    }
    if matches!(
        subevent,
        "PARTY_KILL" | "SPELL_AURA_REMOVED" | "SPELL_AURA_REMOVED_DOSE"
    ) {
        return;
    }
    let group = |flags: u32| flags & AFFILIATION_GROUP != 0;
    if let (Some((guid, flags)), Some((_, dest_flags))) = (source, dest) {
        if guid != 0 && group(dest_flags) {
            engage(lua, guid, flags);
        }
    }
    if let (Some((guid, flags)), Some((_, source_flags))) = (dest, source) {
        if guid != 0 && group(source_flags) {
            engage(lua, guid, flags);
        }
    }
}

/// The per-frame check: a kill once every boss is beaten, a wipe once the group stopped fighting.
pub fn frame(lua: &Lua) {
    let over = {
        let (Some(metadata), Some(mut state)) = (
            lua.app_data_ref::<Metadata>(),
            lua.app_data_mut::<Encounter>(),
        ) else {
            return;
        };
        if !state.active {
            return;
        }
        if beaten(&metadata, &state, false) {
            true
        } else if group_fighting(lua, &metadata) {
            state.saw_fighting = true;
            false
        } else {
            state.saw_fighting
        }
    };
    if over {
        end(lua);
    }
}

/// A loading screen ends the encounter as a wipe.
pub fn entering_world(lua: &Lua) {
    if lua.app_data_ref::<Encounter>().is_some_and(|s| s.active) {
        end(lua);
    }
}
