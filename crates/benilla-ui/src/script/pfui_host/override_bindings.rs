//! ClassicAPI's override bindings (`bindings/Api.cpp`, 4.3.4's `OVERRIDEKEYBINDING`):
//! `SetOverrideBinding(owner, isPriority, "KEY"[, "COMMAND"])` and `ClearOverrideBindings(owner)`,
//! owner-scoped bindings in front of the saved ones, never saved. Per key, two lists, normal and
//! priority, newest first: setting an owner's binding drops its old one from both and puts the
//! new one at the head of its list, an empty command only drops it; the newest priority binding
//! wins, then the newest normal one. A winning override whose command the table lacks leaves its
//! key with no binding. The dispatch reads the overrides through [`overlay`], so a change moves the
//! binding table's generation; `GetBindingAction(key, true)` and `GetBindingByKey(key)` see them.
//!
//! The owner is a frame. pfUI's page master overrides modifier chords of the action keys.
// Deviation: ClassicAPI's typed forms (SetOverrideBindingSpell/Item/Macro/Click) and the typed
// commands they bind are not built; nor is 4.3.4's reduced-key walk in the dispatch, which keeps
// 1.12's exact chord and one retry.

use mlua::{Lua, Value};

use crate::script::keybind::{normalize_binding_key, KeyBinding};
use crate::script::object::frame_handle_of;
use crate::script::Model;
use crate::widget::FrameHandle;

#[derive(Default)]
struct OverrideKey {
    key: String,
    /// `[0]` normal, `[1]` priority, newest first.
    lists: [Vec<(FrameHandle, String)>; 2],
}

#[derive(Default)]
struct Overrides(Vec<OverrideKey>);

impl Overrides {
    fn active(&self, key: &str) -> Option<&str> {
        let entry = self.0.iter().find(|k| k.key == key)?;
        entry.lists[1]
            .first()
            .or_else(|| entry.lists[0].first())
            .map(|(_, command)| command.as_str())
    }
}

/// The key as the binding table writes it, else as given in capitals.
fn key_of(key: &str) -> String {
    normalize_binding_key(key).unwrap_or_else(|| key.to_ascii_uppercase())
}

fn changed(lua: &Lua) {
    if let Some(mut model) = lua.app_data_mut::<Model>() {
        model.keybinds.bump();
    }
}

/// `live` with each overridden key's winning override in place of its saved binding; a winner
/// whose command the table lacks unbinds its key.
pub(crate) fn overlay(lua: &Lua, mut live: Vec<KeyBinding>) -> Vec<KeyBinding> {
    let Some(overrides) = lua.app_data_ref::<Overrides>() else {
        return live;
    };
    if overrides.0.is_empty() {
        return live;
    }
    let model = lua.app_data_ref::<Model>();
    for entry in &overrides.0 {
        let Some(command) = overrides.active(&entry.key) else {
            continue;
        };
        live.retain(|(key, _)| *key != entry.key);
        if model.as_ref().is_some_and(|m| m.keybinds.knows(command)) {
            live.push((entry.key.clone(), command.to_string()));
        }
    }
    live
}

/// The override `key` runs now, if one wins.
pub(crate) fn active(lua: &Lua, key: &str) -> Option<String> {
    lua.app_data_ref::<Overrides>()?
        .active(&key_of(key))
        .map(str::to_string)
}

fn owner(lua: &Lua, value: &Value) -> Option<FrameHandle> {
    let Value::Table(table) = value else {
        return None;
    };
    frame_handle_of(lua, table).ok()
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
    lua.set_app_data(Overrides::default());
    lua.globals().set(
        "SetOverrideBinding",
        lua.create_function(
            |lua, (who, priority, key, command): (Value, Value, Value, Value)| {
                let (Some(owner), Value::String(key)) = (owner(lua, &who), &key) else {
                    return Err(mlua::Error::runtime(
                        "Usage: SetOverrideBinding(owner, isPriority, \"KEY\"[, \"COMMAND\"])",
                    ));
                };
                let key = key_of(&key.to_string_lossy());
                let command = match &command {
                    Value::String(s) => s.to_string_lossy(),
                    _ => String::new(),
                };
                {
                    let mut overrides = lua.app_data_mut::<Overrides>().expect("installed");
                    let at = match overrides.0.iter().position(|k| k.key == key) {
                        Some(at) => at,
                        None => {
                            overrides.0.push(OverrideKey {
                                key,
                                ..Default::default()
                            });
                            overrides.0.len() - 1
                        }
                    };
                    let entry = &mut overrides.0[at];
                    for list in &mut entry.lists {
                        list.retain(|(o, _)| *o != owner);
                    }
                    if !command.is_empty() {
                        entry.lists[usize::from(flag(&priority))].insert(0, (owner, command));
                    }
                }
                changed(lua);
                Ok(())
            },
        )?,
    )?;
    lua.globals().set(
        "ClearOverrideBindings",
        lua.create_function(|lua, who: Value| {
            let Some(owner) = owner(lua, &who) else {
                return Err(mlua::Error::runtime("Usage: ClearOverrideBindings(owner)"));
            };
            let any = {
                let mut overrides = lua.app_data_mut::<Overrides>().expect("installed");
                let mut any = false;
                for entry in &mut overrides.0 {
                    for list in &mut entry.lists {
                        let before = list.len();
                        list.retain(|(o, _)| *o != owner);
                        any |= list.len() != before;
                    }
                }
                any
            };
            if any {
                changed(lua);
            }
            Ok(())
        })?,
    )?;
    // GetBindingAction(key[, checkOverride]) sees the winning override when asked.
    let saved: mlua::Function = lua.globals().get("GetBindingAction")?;
    lua.globals().set(
        "GetBindingAction",
        lua.create_function(move |lua, (key, check): (Value, Value)| {
            if flag(&check) {
                if let Value::String(k) = &key {
                    if let Some(command) = active(lua, &k.to_string_lossy()) {
                        return Ok(Value::String(lua.create_string(command)?));
                    }
                }
            }
            saved.call::<Value>(key)
        })?,
    )?;
    // GetBindingByKey(key): what a press of the key runs, its override first.
    let by_key: mlua::Function = lua.globals().get("GetBindingAction")?;
    lua.globals().set(
        "GetBindingByKey",
        lua.create_function(move |_, key: Value| {
            let action: Value = by_key.call((key, true))?;
            Ok(match action {
                Value::String(s) if s.as_bytes().is_empty() => Value::Nil,
                other => other,
            })
        })?,
    )?;
    Ok(())
}
