//! ClassicAPI's `InteractUnit(unit[, exactMatch])` and
//! `C_PlayerInteractionManager.InteractUnit(unit[, exactMatch[, looseTargeting]])`
//! (`unit/Interact.cpp`): the world right-click on a unit, `InteractWithObjectByClick`
//! (`0x492820`), which selects the unit and runs its interaction: a vendor opens, a corpse loots.
//!
//! The unit is a unit token, or any other text naming the nearest unit whose name starts with it
//! (the whole name with `exactMatch`). "anyinteract" is what the Interact Key uses: the target,
//! while soft interact (`SoftTargetInteract`) picks nothing; "softinteract" names nothing yet;
//! "anyleftinteract" does nothing and answers false. A loose call (the manager's default) with a
//! "soft…" or "any…" token fires `PLAYER_SOFT_TARGET_INTERACTION`. The click is the nameplate
//! click's script call, the same engine leg. `InteractUnit` answers 1 or nil, the manager's
//! form true or false: whether a unit was found to click.
// Deviation: ClassicAPI's "mouseover" and name search also reach game objects and corpses, which
// this client's click script call does not take; they answer false.

use mlua::{Lua, Value};

use crate::script::nameplate::{PlateAction, PlateClick};
use crate::script::{Model, ScriptCall};

/// The unit `token` names for an interaction, by ClassicAPI's rules.
fn resolve(lua: &Lua, token: &str, exact: bool) -> Option<u64> {
    let lower = token.to_ascii_lowercase();
    match lower.as_str() {
        "anyinteract" => return super::auras::unit_guid(lua, "target"),
        "softinteract" | "anyleftinteract" => return None,
        _ => {}
    }
    if let Some(guid) = super::auras::unit_guid(lua, &lower) {
        return Some(guid);
    }
    let as_value = lua.create_string(&lower).map(Value::String).ok();
    if as_value.is_some_and(|v| super::macro_verbs::is_unit_token(&v)) {
        return None;
    }
    nearest_named(lua, &lower, exact)
}

/// The nearest unit whose name starts with `name` (equals it with `exact`), without regard to
/// case: by the reach map's distance, a unit with none last.
fn nearest_named(lua: &Lua, name: &str, exact: bool) -> Option<u64> {
    let candidates: Vec<u64> = super::units::named_units(lua, |unit| {
        let unit = unit.to_ascii_lowercase();
        if exact {
            unit == name
        } else {
            unit.starts_with(name)
        }
    });
    let model = lua.app_data_ref::<Model>()?;
    candidates.into_iter().min_by(|a, b| {
        let d = |g: &u64| model.unit_reach.get(g).map_or(f64::INFINITY, |r| r.dist_sq);
        d(a).total_cmp(&d(b))
    })
}

/// Click `guid` as the world's right button does; whether a unit was there to click.
fn click(lua: &Lua, guid: u64) -> bool {
    let may_cast = crate::script::protected::select_may_cast(lua);
    let call = ScriptCall::PlateClick(PlateClick {
        guid,
        action: PlateAction::Interact,
        scripted: true,
        may_cast,
    });
    if let Err(e) = crate::script::calls::submit(lua, call) {
        lua.app_data_mut::<Model>()
            .expect("model")
            .record_lua_error(&e);
    }
    true
}

fn token_arg(value: &Value, usage: &str) -> mlua::Result<String> {
    match value {
        Value::String(s) => Ok(s.to_string_lossy()),
        Value::Integer(_) | Value::Number(_) => Ok(value.to_string()?),
        _ => Err(mlua::Error::runtime(usage.to_string())),
    }
}

fn flag(value: &Value) -> bool {
    match value {
        Value::Nil | Value::Boolean(false) => false,
        Value::Integer(n) => *n != 0,
        Value::Number(n) => *n != 0.0,
        _ => true,
    }
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.globals().set(
        "InteractUnit",
        lua.create_function(|lua, (unit, exact): (Value, Value)| {
            let token = token_arg(&unit, "Usage: InteractUnit(unitToken[, exactMatch])")?;
            let found = resolve(lua, &token, flag(&exact)).is_some_and(|guid| click(lua, guid));
            Ok(if found { Value::Integer(1) } else { Value::Nil })
        })?,
    )?;
    let manager = match lua.globals().get::<Value>("C_PlayerInteractionManager")? {
        Value::Table(t) => t,
        _ => lua.create_table()?,
    };
    manager.set(
        "InteractUnit",
        lua.create_function(|lua, (unit, exact, loose): (Value, Value, Value)| {
            let token = token_arg(
                &unit,
                "Usage: C_PlayerInteractionManager.InteractUnit(unit[, exactMatch[, looseTargeting]])",
            )?;
            let loose = loose.is_nil() || flag(&loose);
            let Some(guid) = resolve(lua, &token, flag(&exact)) else {
                return Ok(false);
            };
            let lower = token.to_ascii_lowercase();
            if loose && (lower.starts_with("soft") || lower.starts_with("any")) {
                super::fire_pfui_event(lua, "PLAYER_SOFT_TARGET_INTERACTION", vec![]);
            }
            Ok(click(lua, guid))
        })?,
    )?;
    lua.globals().set("C_PlayerInteractionManager", manager)?;
    Ok(())
}
