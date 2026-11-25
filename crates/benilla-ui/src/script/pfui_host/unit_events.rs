//! Changes for additional tokens are measured against their actual GUID snapshots.
//!
//! Follows ClassicAPI's `frame/UnitEvent.cpp`.

use super::{resolve, state, UnitState};
use mlua::Lua;
use std::collections::BTreeMap;

pub(super) type Snapshot = BTreeMap<String, (Option<UnitState>, Option<u64>)>;

pub(super) fn snapshot(lua: &Lua) -> Snapshot {
    super::super::query_tokens(lua)
        .into_iter()
        .map(|token| {
            let guid = resolve(lua, &token).flatten();
            let value = guid.and_then(|guid| state(lua, guid));
            let target = lua
                .app_data_ref::<super::Metadata>()
                .and_then(|data| guid.and_then(|guid| data.targets.get(&guid).copied()));
            (token, (value, target))
        })
        .collect()
}

/// The unit events a unit's change from `before` to `after` raises, its target from `old_target`
/// to `target`.
fn changes(
    before: Option<&UnitState>,
    old_target: Option<u64>,
    after: Option<&UnitState>,
    target: Option<u64>,
) -> Vec<&'static str> {
    let mut events = Vec::new();
    if old_target != target {
        events.push("UNIT_TARGET");
    }
    macro_rules! changed {
        ($field:ident, $event:literal) => {
            if before.map(|state| &state.$field) != after.map(|state| &state.$field) {
                events.push($event);
            }
        };
    }
    changed!(name, "UNIT_NAME_UPDATE");
    changed!(health, "UNIT_HEALTH");
    changed!(max_health, "UNIT_MAXHEALTH");
    changed!(level, "UNIT_LEVEL");
    changed!(flags, "UNIT_FLAGS");
    changed!(dynamic_flags, "UNIT_DYNAMIC_FLAGS");
    changed!(player_flags, "PLAYER_FLAGS_CHANGED");
    changed!(is_connected, "UNIT_CONNECTION");
    changed!(quest_log, "UNIT_QUEST_LOG_CHANGED");
    let faction = |state: &UnitState| {
        (
            state.reaction,
            state.pvp,
            state.tapped,
            state.tapped_by_player,
            state.faction_group.clone(),
        )
    };
    if before.map(faction) != after.map(faction) {
        events.push("UNIT_FACTION");
    }
    events
}

pub(super) fn dispatch(lua: &Lua, before: Snapshot, after: Snapshot) {
    for (token, (after, target)) in after {
        let (before, old_target) = before.get(&token).cloned().unwrap_or_default();
        for event in changes(before.as_ref(), old_target, after.as_ref(), target) {
            crate::script::pfui_host::fire_pfui_event(
                lua,
                event,
                vec![crate::script::ScriptValue::Str(token.clone())],
            );
        }
    }
}

/// The units frames registered by GUID ([`crate::script::pfui_host::guid_events`]), as
/// [`snapshot`] takes the tokens'.
pub(super) type GuidSnapshot = BTreeMap<u64, (Option<UnitState>, Option<u64>)>;

pub(super) fn guid_snapshot(lua: &Lua) -> GuidSnapshot {
    crate::script::pfui_host::guid_events::watched(lua)
        .into_iter()
        .map(|guid| {
            let target = lua
                .app_data_ref::<super::Metadata>()
                .and_then(|data| data.targets.get(&guid).copied());
            (guid, (state(lua, guid), target))
        })
        .collect()
}

/// Each watched unit's changes to the frames that registered its GUID, whether or not a token
/// names it; one a token named this frame already went out with that token's walk.
pub(super) fn guid_dispatch(lua: &Lua, before: GuidSnapshot, after: GuidSnapshot) {
    for (guid, (after, target)) in after {
        let (before, old_target) = before.get(&guid).cloned().unwrap_or_default();
        for event in changes(before.as_ref(), old_target, after.as_ref(), target) {
            crate::script::pfui_host::guid_events::signal(lua, event, guid, &[]);
        }
    }
}
