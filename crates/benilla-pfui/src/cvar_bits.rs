//! ClassicAPI's `C_CVar.GetCVarBitfield(name, index)` and `SetCVarBitfield(name, index, value)`
//! (`cvar/Bitfield.cpp`): one bit, 1 to 64, of a CVar's value read as an unsigned decimal (leading
//! blanks skipped, the digits up to the first other character, saturating). A name no CVar has, a
//! non-string name, a non-number index or one out of range gets nil from the getter and false from
//! the setter; the setter writes only a changed value and answers true. The value is 64 bits wide,
//! more than a Lua number holds exactly, hence Rust.

use mlua::{Lua, Table, Value};

/// The CVar's value as `Bitfield.cpp`'s `ValueOf` reads it, `None` for no such CVar.
fn value_of(lua: &Lua, name: &str) -> mlua::Result<Option<u64>> {
    let get: mlua::Function = lua.globals().get("GetCVar")?;
    let Some(text) = get.call::<Option<String>>(name)? else {
        return Ok(None);
    };
    let mut value: u64 = 0;
    for digit in text
        .trim_start_matches([' ', '\t'])
        .bytes()
        .take_while(u8::is_ascii_digit)
    {
        let Some(next) = value
            .checked_mul(10)
            .and_then(|v| v.checked_add(u64::from(digit - b'0')))
        else {
            return Ok(Some(u64::MAX));
        };
        value = next;
    }
    Ok(Some(value))
}

/// The name and the bit for a call, or `None` for arguments the calls refuse.
fn args(lua: &Lua, name: &Value, index: &Value) -> mlua::Result<Option<(String, u64)>> {
    let name = match name {
        Value::String(s) => s.to_string_lossy(),
        Value::Integer(_) | Value::Number(_) => name.to_string()?,
        _ => return Ok(None),
    };
    let Some(index) = lua.coerce_number(index.clone())? else {
        return Ok(None);
    };
    if !(1.0..=64.0).contains(&index) || index.fract() != 0.0 {
        return Ok(None);
    }
    Ok(Some((name, 1u64 << (index as u32 - 1))))
}

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let api: Table = lua.globals().get("C_CVar")?;
    api.set(
        "GetCVarBitfield",
        lua.create_function(|lua, (name, index): (Value, Value)| {
            let Some((name, bit)) = args(lua, &name, &index)? else {
                return Ok(None);
            };
            Ok(value_of(lua, &name)?.map(|value| value & bit != 0))
        })?,
    )?;
    api.set(
        "SetCVarBitfield",
        lua.create_function(|lua, (name, index, set): (Value, Value, Value)| {
            let Some((name, bit)) = args(lua, &name, &index)? else {
                return Ok(false);
            };
            let Some(before) = value_of(lua, &name)? else {
                return Ok(false);
            };
            let after = if set.as_boolean().unwrap_or(!set.is_nil()) {
                before | bit
            } else {
                before & !bit
            };
            if after != before {
                let set_cvar: mlua::Function = lua.globals().get("SetCVar")?;
                set_cvar.call::<()>((name, after.to_string()))?;
            }
            Ok(true)
        })?,
    )?;
    Ok(())
}
