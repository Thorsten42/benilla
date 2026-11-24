//! `frame:SetPassThroughButtons(...)`, ClassicAPI's half of retail's (`frame/MouseInput.cpp`): the
//! named mouse buttons go through the frame. A press of one skips it, so the frame gets nothing for
//! that press and the next mouse frame under the cursor takes it, or the world when none does; its
//! other buttons and its hover are untouched. Each call replaces the frame's set, and no argument
//! clears it. A name that is no button is accepted and passes nothing; an argument that is not a
//! string raises the usage error.
//!
//! The press router asks [`passes`] for the frame its walk picked ([`crate::script::pointer`]),
//! and only when no capture holds the press: a press while another button is held goes to the
//! frame already pressed, as the engine routes it, and the walk does not run.

use std::collections::HashMap;

use mlua::{Lua, MultiValue, Table, Value};

use crate::script::object::frame_handle_of;
use crate::widget::FrameHandle;

/// The frames that let buttons through, each with its set as a mask ([`bit`]).
#[derive(Default)]
struct PassThrough(HashMap<FrameHandle, u8>);

/// The mask bit of a WoW button name, `None` for a name that is no button.
fn bit(button: &str) -> Option<u8> {
    Some(match button {
        "LeftButton" => 1,
        "RightButton" => 2,
        "MiddleButton" => 4,
        "Button4" => 8,
        "Button5" => 16,
        _ => return None,
    })
}

/// Whether `frame` lets a press of `button` through.
pub(crate) fn passes(lua: &Lua, frame: FrameHandle, button: &str) -> bool {
    let Some(bit) = bit(button) else {
        return false;
    };
    lua.app_data_ref::<PassThrough>()
        .is_some_and(|set| set.0.get(&frame).is_some_and(|mask| mask & bit != 0))
}

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(PassThrough::default());
    let frames: Table = lua.named_registry_value(super::super::REG_FRAME_METHODS)?;
    frames.set(
        "SetPassThroughButtons",
        lua.create_function(|lua, (this, buttons): (Table, MultiValue)| {
            let h = frame_handle_of(lua, &this)?;
            let mut mask = 0u8;
            for button in buttons {
                let Value::String(name) = button else {
                    return Err(mlua::Error::runtime(
                        "Usage: self:SetPassThroughButtons([button1, ...])",
                    ));
                };
                mask |= bit(&name.to_string_lossy()).unwrap_or(0);
            }
            let mut set = lua.app_data_mut::<PassThrough>().expect("installed");
            if mask == 0 {
                set.0.remove(&h);
            } else {
                set.0.insert(h, mask);
            }
            Ok(())
        })?,
    )?;
    Ok(())
}
