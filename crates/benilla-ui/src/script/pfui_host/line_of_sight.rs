//! ClassicAPI's `UnitInLineOfSight(unit)` (`unit/LineOfSight.cpp`): whether the player sees the
//! unit through the world's geometry, the server's one-segment check with each end raised by its
//! unit's height. True when clear (and for the player itself), false when blocked or past
//! 150 yards, nil when the token names no unit (or a GameObject).
//!
//! The VM cannot trace the world, so the host traces every unit near the player and hands the
//! verdicts over ([`set`]); a unit the host did not trace (past 150 yards) reads as blocked.
// Deviation: the verdict is at most 100 ms old (the host's pass), where ClassicAPI traces on the
// call.

use std::collections::HashMap;

use mlua::{Lua, Value};

use crate::script::Model;

#[derive(Default)]
struct LineOfSight {
    /// Each traced unit's verdict, clear or blocked.
    clear: HashMap<u64, bool>,
}

/// The host's newest pass: every unit within 150 yards, and whether the line to it is clear.
pub fn set(lua: &Lua, clear: HashMap<u64, bool>) {
    if let Some(mut state) = lua.app_data_mut::<LineOfSight>() {
        state.clear = clear;
    }
}

/// A GameObject's GUID (high word 0xF110), which ClassicAPI answers nil for.
fn is_game_object(guid: u64) -> bool {
    guid >> 52 == 0xF11
}

fn in_sight(lua: &Lua, token: &str) -> Option<bool> {
    let lower = token.to_ascii_lowercase();
    let guid = super::auras::unit_guid(lua, &lower)?;
    if is_game_object(guid) {
        return None;
    }
    let me = lua.app_data_ref::<Model>().map(|m| m.unit_guids.player);
    if me == Some(guid) {
        return Some(true);
    }
    let state = lua.app_data_ref::<LineOfSight>()?;
    Some(state.clear.get(&guid).copied().unwrap_or(false))
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(LineOfSight::default());
    lua.globals().set(
        "UnitInLineOfSight",
        lua.create_function(|lua, unit: Value| {
            let Value::String(token) = &unit else {
                return Err(mlua::Error::runtime(r#"Usage: UnitInLineOfSight("unit")"#));
            };
            let token = token.to_str()?.to_string();
            Ok(in_sight(lua, &token).map_or(Value::Nil, Value::Boolean))
        })?,
    )?;
    Ok(())
}
