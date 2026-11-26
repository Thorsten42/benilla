//! ClassicAPI's `UnitInLineOfSight(unit)` (`unit/LineOfSight.cpp`): whether the player sees the
//! unit through the world's geometry, the server's one-segment check with each end raised by its
//! unit's height. True when clear (and for the player itself), false when blocked or past
//! 150 yards, nil when the token names no unit (or a GameObject).
//!
//! The VM cannot trace the world, so the host traces the units the VM asked about in the last
//! [`WANTED_FOR`] seconds ([`wanted`]) and hands the verdicts over ([`set`]); nothing asked costs
//! no trace.
// Deviation: the verdict is at most 100 ms old (the host's pass), where ClassicAPI traces on the
// call; a unit's first ask reads nil until its first trace lands.

use std::collections::HashMap;

use mlua::{Lua, Value};

use crate::script::Model;

/// How long an ask keeps its unit traced: past any addon's poll (pfUI's target indicator asks
/// every 0.2 s).
const WANTED_FOR: f64 = 2.0;

#[derive(Default)]
struct LineOfSight {
    /// Each traced unit's verdict, clear or blocked.
    clear: HashMap<u64, bool>,
    /// Each unit asked about, with the clock of its last ask.
    asked: HashMap<u64, f64>,
}

fn now(lua: &Lua) -> f64 {
    lua.app_data_ref::<Model>().map_or(0.0, |m| m.now)
}

/// The units the host should trace: those asked about in the last [`WANTED_FOR`] seconds.
pub fn wanted(lua: &Lua) -> Vec<u64> {
    let now = now(lua);
    let Some(mut state) = lua.app_data_mut::<LineOfSight>() else {
        return Vec::new();
    };
    state.asked.retain(|_, at| now - *at <= WANTED_FOR);
    state.asked.keys().copied().collect()
}

/// The host's newest pass: each wanted unit it found, and whether the line to it is clear (false
/// past 150 yards).
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
    let now = now(lua);
    let mut state = lua.app_data_mut::<LineOfSight>()?;
    state.asked.insert(guid, now);
    state.clear.get(&guid).copied()
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
