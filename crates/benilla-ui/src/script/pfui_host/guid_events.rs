//! `frame:RegisterGUIDEvent(event, guid, ...)`, `UnregisterGUIDEvent(event)` and
//! `IsGUIDEventRegistered(event)`: unit events by GUID, ClassicAPI's extension
//! (`frame/GuidEvent.cpp`).
//!
//! A GUID registration is kept apart from the frame's event registrations: `RegisterEvent`,
//! `RegisterUnitEvent`, `UnregisterEvent` and `IsEventRegistered` never see it, and the event walk
//! does not reach it. Each unit event for a unit goes to the frames that registered its GUID once,
//! with `arg1` the GUID ([`signal`]): the walk hands its token's unit over after its own listeners
//! ([`crate::script::tick::fire_event_into`]), and the layer's feeds signal the units no token
//! names (auras, powers, visibility). A unit with two tokens fires its event once per token, so a
//! GUID's listeners take one delivery of an event per frame. `UnregisterAllEvents` drops the
//! frame's GUID registrations too.

use std::collections::HashMap;

use mlua::{Lua, MultiValue, ObjectLike, Table, Value};

use crate::script::object::frame_handle_of;
use crate::script::{Model, ScriptValue};
use crate::widget::FrameHandle;

#[derive(Default)]
struct GuidEvents {
    /// Each frame's GUIDs for each event, in the order given.
    by_frame: HashMap<(FrameHandle, String), Vec<u64>>,
    /// The `GetTime()` frame each event last went out for a GUID.
    delivered: HashMap<(String, u64), f64>,
}

/// Whether ClassicAPI takes `event` as a unit event.
fn is_unit_event(event: &str) -> bool {
    event.starts_with("UNIT_") || event == "PLAYER_FLAGS_CHANGED"
}

/// A GUID as `UnitGUID` writes it.
pub(crate) fn guid_string(guid: u64) -> String {
    format!("0x{guid:016X}")
}

fn parse_guid(text: &str) -> Option<u64> {
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u64::from_str_radix(hex, 16).ok().filter(|guid| *guid != 0)
}

/// Whether any frame registered a GUID for `event`, so a walk with no listener of its own still
/// runs for them.
pub(crate) fn listened(lua: &Lua, event: &str) -> bool {
    is_unit_event(event)
        && lua
            .app_data_ref::<GuidEvents>()
            .is_some_and(|state| state.by_frame.keys().any(|(_, e)| e == event))
}

/// Whether some frame listens for `event` on `guid`.
pub(crate) fn listens(lua: &Lua, event: &str, guid: u64) -> bool {
    lua.app_data_ref::<GuidEvents>().is_some_and(|state| {
        state
            .by_frame
            .iter()
            .any(|((_, e), guids)| e == event && guids.contains(&guid))
    })
}

/// The unit a walk of `event` names in its first argument, when a frame registered its GUID for
/// the event.
pub(crate) fn unit_of(lua: &Lua, event: &str, arg1: Option<&ScriptValue>) -> Option<u64> {
    if !listened(lua, event) {
        return None;
    }
    let Some(ScriptValue::Str(token)) = arg1 else {
        return None;
    };
    let guid = super::auras::unit_guid(lua, token)?;
    listens(lua, event, guid).then_some(guid)
}

/// The GUIDs any frame registered for some event.
pub(crate) fn watched(lua: &Lua) -> Vec<u64> {
    let mut guids: Vec<u64> = lua
        .app_data_ref::<GuidEvents>()
        .map(|state| state.by_frame.values().flatten().copied().collect())
        .unwrap_or_default();
    guids.sort_unstable();
    guids.dedup();
    guids
}

/// The unit event `event` for `guid`, to the frames that registered the GUID: once per frame of
/// the clock, `arg1` the GUID and `rest` after it. `event` is set around the calls as a walk sets
/// it.
pub(crate) fn signal(lua: &Lua, event: &str, guid: u64, rest: &[ScriptValue]) {
    let now = crate::script::clock::now(lua);
    let frames: Vec<FrameHandle> = {
        let Some(mut state) = lua.app_data_mut::<GuidEvents>() else {
            return;
        };
        let frames: Vec<FrameHandle> = state
            .by_frame
            .iter()
            .filter(|((_, e), guids)| e == event && guids.contains(&guid))
            .map(|((frame, _), _)| *frame)
            .collect();
        if frames.is_empty() {
            return;
        }
        let key = (event.to_string(), guid);
        if state.delivered.get(&key) == Some(&now) {
            return;
        }
        state.delivered.insert(key, now);
        frames
    };
    let mut args = Vec::with_capacity(rest.len() + 1);
    args.push(ScriptValue::Str(guid_string(guid)));
    args.extend_from_slice(rest);
    let globals = lua.globals();
    let saved = globals.get::<Value>("event").ok();
    let _ = globals.set("event", event);
    for frame in frames {
        let id = lua
            .app_data_ref::<Model>()
            .and_then(|model| model.frame_to_id.get(&frame).copied());
        if let Some(id) = id {
            if let Err(e) = crate::script::event::fire_event_handler(lua, id, &args) {
                lua.app_data_mut::<Model>()
                    .expect("model")
                    .record_lua_error(&e);
            }
        }
    }
    if let Some(saved) = saved {
        let _ = globals.set("event", saved);
    }
}

/// "<name>:<method>(): <message>", the frame's name, else its type, as `RegisterUnitEvent`'s
/// errors read.
fn method_error(frame: &Table, method: &str, message: &str) -> mlua::Error {
    let name = frame
        .call_method::<Option<String>>("GetName", ())
        .ok()
        .flatten()
        .or_else(|| frame.call_method::<String>("GetObjectType", ()).ok())
        .unwrap_or_default();
    mlua::Error::runtime(format!("{name}:{method}(): {message}"))
}

const REGISTER_USAGE: &str = "Usage: (\"event\", \"guid1\" [, \"guid2\", ...])";

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(GuidEvents::default());
    let frames: Table = lua.named_registry_value(super::super::REG_FRAME_METHODS)?;
    // The GUIDs replace the frame's earlier ones for the event; true when that changed them.
    frames.set(
        "RegisterGUIDEvent",
        lua.create_function(|lua, (this, args): (Table, MultiValue)| {
            let mut args = args.into_iter();
            let Some(Value::String(event)) = args.next() else {
                return Err(method_error(&this, "RegisterGUIDEvent", REGISTER_USAGE));
            };
            let event = event.to_string_lossy();
            if !is_unit_event(&event) {
                return Err(method_error(
                    &this,
                    "RegisterGUIDEvent",
                    &format!("Attempt to register unknown event \"{event}\""),
                ));
            }
            // GUIDs up to the first value that is not a string, each once; a string that is no
            // GUID is dropped.
            let mut guids = Vec::new();
            for value in args {
                let text = match &value {
                    Value::String(s) => s.to_string_lossy(),
                    Value::Integer(_) | Value::Number(_) => value.to_string()?,
                    _ => break,
                };
                if let Some(guid) = parse_guid(&text).filter(|g| !guids.contains(g)) {
                    guids.push(guid);
                }
            }
            if guids.is_empty() {
                return Err(method_error(&this, "RegisterGUIDEvent", REGISTER_USAGE));
            }
            let frame = frame_handle_of(lua, &this)?;
            let mut state = lua.app_data_mut::<GuidEvents>().expect("installed");
            let old = state.by_frame.insert((frame, event), guids.clone());
            Ok(old.as_ref() != Some(&guids))
        })?,
    )?;
    frames.set(
        "UnregisterGUIDEvent",
        lua.create_function(|lua, (this, event): (Table, Value)| {
            let Value::String(event) = event else {
                return Err(mlua::Error::runtime(
                    "bad argument #2 to 'UnregisterGUIDEvent' (Usage: self:UnregisterGUIDEvent(eventName))",
                ));
            };
            let frame = frame_handle_of(lua, &this)?;
            lua.app_data_mut::<GuidEvents>()
                .expect("installed")
                .by_frame
                .remove(&(frame, event.to_string_lossy()));
            Ok(())
        })?,
    )?;
    // isRegistered, then the GUIDs in their order, or a nil second return.
    frames.set(
        "IsGUIDEventRegistered",
        lua.create_function(|lua, (this, event): (Table, Value)| {
            let Value::String(event) = event else {
                return Err(mlua::Error::runtime(
                    "bad argument #2 to 'IsGUIDEventRegistered' (Usage: local isRegistered, (guids)* = self:IsGUIDEventRegistered(eventName))",
                ));
            };
            let frame = frame_handle_of(lua, &this)?;
            let guids = lua
                .app_data_ref::<GuidEvents>()
                .expect("installed")
                .by_frame
                .get(&(frame, event.to_string_lossy()))
                .cloned();
            let Some(guids) = guids else {
                return Ok(MultiValue::from_vec(vec![Value::Boolean(false), Value::Nil]));
            };
            let mut out = vec![Value::Boolean(true)];
            for guid in guids {
                out.push(Value::String(lua.create_string(guid_string(guid))?));
            }
            Ok(MultiValue::from_vec(out))
        })?,
    )?;
    // UnregisterAllEvents drops the GUID registrations too.
    let unregister_all: mlua::Function = frames.get("UnregisterAllEvents")?;
    frames.set(
        "UnregisterAllEvents",
        lua.create_function(move |lua, (this, rest): (Table, MultiValue)| {
            unregister_all.call::<()>((this.clone(), rest))?;
            let frame = frame_handle_of(lua, &this)?;
            lua.app_data_mut::<GuidEvents>()
                .expect("installed")
                .by_frame
                .retain(|(f, _), _| *f != frame);
            Ok(())
        })?,
    )?;
    Ok(())
}
