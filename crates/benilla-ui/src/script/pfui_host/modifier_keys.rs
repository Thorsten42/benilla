//! ClassicAPI's left and right modifier keys (`input/Modifier.cpp`): `IsLeftShiftKeyDown`,
//! `IsRightShiftKeyDown`, `IsLeftControlKeyDown`, `IsRightControlKeyDown`, `IsLeftAltKeyDown` and
//! `IsRightAltKeyDown`, each 1 or nil, and `MODIFIER_STATE_CHANGED(key, down)` for each side that
//! changed, `key` "LSHIFT", "RSHIFT", "LCTRL", "RCTRL", "LALT" or "RALT" and `down` 1 or 0, in that
//! order. The host feeds the held sides once a frame ([`set_sides`]).

use mlua::Lua;

use crate::script::ScriptValue;

const KEYS: [&str; 6] = ["LSHIFT", "RSHIFT", "LCTRL", "RCTRL", "LALT", "RALT"];

#[derive(Default)]
struct Sides {
    held: u8,
}

/// The held sides now, bit `i` for `KEYS[i]`; each change fires `MODIFIER_STATE_CHANGED`.
pub fn set_sides(lua: &Lua, held: u8) {
    let held = held & 0x3f;
    let changed = {
        let Some(mut sides) = lua.app_data_mut::<Sides>() else {
            return;
        };
        let changed = sides.held ^ held;
        sides.held = held;
        changed
    };
    for (bit, key) in KEYS.iter().enumerate() {
        if changed & (1 << bit) != 0 {
            super::fire_pfui_event(
                lua,
                "MODIFIER_STATE_CHANGED",
                vec![
                    ScriptValue::Str((*key).to_string()),
                    ScriptValue::Int(i64::from((held >> bit) & 1)),
                ],
            );
        }
    }
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Sides::default());
    for (bit, name) in [
        "IsLeftShiftKeyDown",
        "IsRightShiftKeyDown",
        "IsLeftControlKeyDown",
        "IsRightControlKeyDown",
        "IsLeftAltKeyDown",
        "IsRightAltKeyDown",
    ]
    .into_iter()
    .enumerate()
    {
        lua.globals().set(
            name,
            lua.create_function(move |lua, ()| {
                let held = lua.app_data_ref::<Sides>().map_or(0, |s| s.held);
                Ok(crate::script::binding_abi::flag(held & (1 << bit) != 0))
            })?,
        )?;
    }
    Ok(())
}
