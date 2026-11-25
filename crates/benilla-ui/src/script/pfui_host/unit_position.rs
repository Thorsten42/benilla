//! ClassicAPI's `UnitPosition(unit)` and `C_PlayerInfo.UnitPosition` (`unit/Position.cpp`): the
//! world position of any unit in view, in retail's order: posY (west), posX (north), posZ (up),
//! and the map ID as the instance. Nil when the unit is not in view. The host reads the position
//! at the call ([`UnitPositionQuery`]). pfUI's radar and its Prayer of Healing prediction read it.

use mlua::{Lua, MultiValue, Value};

use crate::script::live::{ask, LiveQuery};

/// The unit `guid`'s world position (x north, y west, z up) and the map it is on, `None` when the
/// client holds no position for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitPositionQuery(pub u64);

impl LiveQuery for UnitPositionQuery {
    type Answer = Option<([f32; 3], u32)>;
}

fn unit_position(lua: &Lua, unit: Value) -> mlua::Result<MultiValue> {
    let Value::String(token) = unit else {
        return Err(mlua::Error::runtime(r#"Usage: UnitPosition("unit")"#));
    };
    let token = token.to_str()?.to_ascii_lowercase();
    let at = super::auras::unit_guid(lua, &token)
        .and_then(|guid| ask(lua, &UnitPositionQuery(guid)).flatten());
    let Some(([x, y, z], map)) = at else {
        return Ok(MultiValue::from_vec(vec![Value::Nil]));
    };
    Ok(MultiValue::from_vec(vec![
        Value::Number(f64::from(y)),
        Value::Number(f64::from(x)),
        Value::Number(f64::from(z)),
        Value::Integer(i64::from(map)),
    ]))
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let f = lua.create_function(unit_position)?;
    lua.globals().set("UnitPosition", f.clone())?;
    let info = match lua.globals().get::<Value>("C_PlayerInfo")? {
        Value::Table(t) => t,
        _ => lua.create_table()?,
    };
    info.set("UnitPosition", f)?;
    lua.globals().set("C_PlayerInfo", info)
}
