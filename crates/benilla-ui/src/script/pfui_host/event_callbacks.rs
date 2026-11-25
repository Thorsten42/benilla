//! The engine half of ClassicAPI's event callbacks (`event/Callback.cpp`): after an event's frame
//! walk, the eight callback events go to the dispatch `benilla-pfui`'s `callbacks.lua` keeps under
//! [`EVENT_CALLBACKS`], which runs the registered callbacks. The walk runs for these events even
//! with no frame listening.

use mlua::Lua;

use crate::script::ScriptValue;

/// Registry key of the callbacks' dispatch, `function(event, ...)`.
pub const EVENT_CALLBACKS: &str = "benilla.pfui.event_callbacks";

/// The callback events (`Callback.cpp`'s `kNames`), as the engine fires them.
const EVENTS: [&str; 8] = [
    "MINIMAP_PING",
    "COMBAT_LOG_APPLY_FILTER_SETTINGS",
    "ENCOUNTER_STATE_CHANGED",
    "TOOLTIP_SHOW_ITEM_COMPARISON",
    "COMBAT_LOG_EVENT",
    "COMBAT_LOG_EVENT_UNFILTERED",
    "COMBAT_LOG_EVENT_INTERNAL_UNFILTERED",
    "COMBAT_LOG_REFILTER_ENTRIES",
];

/// Whether `event` takes callbacks and a dispatch is installed.
pub(crate) fn listened(lua: &Lua, event: &str) -> bool {
    EVENTS.contains(&event)
        && lua
            .named_registry_value::<Option<mlua::Function>>(EVENT_CALLBACKS)
            .ok()
            .flatten()
            .is_some()
}

/// Run `event`'s callbacks with its values; a dispatch error is the VM's to record.
pub(crate) fn dispatch(lua: &Lua, event: &str, args: &[ScriptValue]) {
    if !EVENTS.contains(&event) {
        return;
    }
    let Ok(Some(dispatch)) = lua.named_registry_value::<Option<mlua::Function>>(EVENT_CALLBACKS)
    else {
        return;
    };
    let mut values = vec![mlua::Value::String(match lua.create_string(event) {
        Ok(s) => s,
        Err(_) => return,
    })];
    values.extend(
        args.iter()
            .filter_map(|value| value.clone().into_lua(lua).ok()),
    );
    if let Err(e) = dispatch.call::<()>(mlua::MultiValue::from_vec(values)) {
        if let Some(mut model) = lua.app_data_mut::<crate::script::Model>() {
            model.record_lua_error(&e);
        }
    }
}
