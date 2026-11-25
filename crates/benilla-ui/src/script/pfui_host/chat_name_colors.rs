//! ClassicAPI's sender names in their class colour, per chat type (`chat/ColorNameByClass.cpp`):
//! `SetChatColorNameByClass(chatType, colorNameByClass)` sets the flag of a chat-colour registry
//! entry (a chat type such as "GUILD", or a channel "CHANNEL1".."CHANNEL10"; the case is not
//! important) and fires `UPDATE_CHAT_COLOR_NAME_BY_CLASS(chatType, colorNameByClass)`; a name the
//! registry lacks changes nothing. The flag rides each registry line of `chat-cache.txt` as a
//! trailing Y or N, which the app reads and writes ([`load`], [`flag`]), and every entry is
//! announced when the chat settings load ([`announce`]). ClassicAPI's chat frame does the
//! colouring.

use std::collections::HashMap;

use mlua::{Lua, Value};

use crate::script::{Model, ScriptValue};

#[derive(Default)]
struct NameColors(HashMap<String, bool>);

/// The registry's entry named `name`, without regard to case.
fn entry(lua: &Lua, name: &str) -> Option<String> {
    let model = lua.app_data_ref::<Model>()?;
    model
        .chat_colors
        .iter()
        .find(|c| c.name.eq_ignore_ascii_case(name))
        .map(|c| c.name.clone())
}

fn fire(lua: &Lua, name: &str, on: bool) {
    super::fire_pfui_event(
        lua,
        "UPDATE_CHAT_COLOR_NAME_BY_CLASS",
        vec![ScriptValue::Str(name.to_string()), ScriptValue::Bool(on)],
    );
}

/// Whether the entry `name` shows sender names in their class colour.
pub fn flag(lua: &Lua, name: &str) -> bool {
    lua.app_data_ref::<NameColors>()
        .and_then(|flags| flags.0.get(&name.to_ascii_uppercase()).copied())
        .unwrap_or(false)
}

/// Seat the flags the file holds; a name the registry lacks is dropped, no event fires.
pub fn load(lua: &Lua, flags: impl IntoIterator<Item = (String, bool)>) {
    let flags: Vec<(String, bool)> = flags
        .into_iter()
        .filter_map(|(name, on)| entry(lua, &name).map(|name| (name.to_ascii_uppercase(), on)))
        .collect();
    if let Some(mut store) = lua.app_data_mut::<NameColors>() {
        store.0.clear();
        store.0.extend(flags);
    }
}

/// `UPDATE_CHAT_COLOR_NAME_BY_CLASS` for every registry entry, as the chat settings load.
pub fn announce(lua: &Lua) {
    let names: Vec<String> = lua
        .app_data_ref::<Model>()
        .map(|model| model.chat_colors.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();
    for name in names {
        let on = flag(lua, &name);
        fire(lua, &name, on);
    }
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(NameColors::default());
    lua.globals().set(
        "SetChatColorNameByClass",
        lua.create_function(|lua, (kind, on): (Value, Value)| {
            let kind = match kind {
                Value::String(s) => s.to_string_lossy(),
                Value::Integer(_) | Value::Number(_) => kind.to_string()?,
                _ => {
                    return Err(mlua::Error::runtime(
                        "Usage: SetChatColorNameByClass(chatType, colorNameByClass)",
                    ))
                }
            };
            let on = match on {
                Value::Nil | Value::Boolean(false) => false,
                Value::Integer(n) => n != 0,
                Value::Number(n) => n != 0.0,
                _ => true,
            };
            let Some(name) = entry(lua, &kind) else {
                return Ok(());
            };
            lua.app_data_mut::<NameColors>()
                .expect("installed")
                .0
                .insert(name.to_ascii_uppercase(), on);
            if let Some(mut model) = lua.app_data_mut::<Model>() {
                model.chat_colors_changed = true;
            }
            fire(lua, &kind, on);
            Ok(())
        })?,
    )?;
    Ok(())
}
