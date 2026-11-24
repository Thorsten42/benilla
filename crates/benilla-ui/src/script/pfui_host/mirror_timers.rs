//! `GetMirrorTimerInfo(index)` and `GetMirrorTimerProgress(timer)`, ClassicAPI's readers for the
//! fatigue, breath and feign-death bars (`unit/MirrorTimer.cpp`). 1.12's handler (`0x5e7990`)
//! only signals the edges; ClassicAPI keeps 3.3.5's per-type slot beside it, and so does this:
//! a start writes its type's slot and stamps the time, a pause writes nothing (vmangos and
//! Tortoise never send one, restating with a start instead), a stop empties the slot. The app
//! records each edge before it raises it, as ClassicAPI's packet subscriber runs before the
//! engine's handler, so a `MIRROR_TIMER_*` handler already reads the new state.
//!
//! ClassicAPI also empties every slot on `PLAYER_LEAVING_WORLD`, after `StopMirrorTimers`
//! (`0x5e9b90`) signalled a stop for each type without a packet; here that release's stops come
//! through [`stop`], which empties the slots the same way. Deviations: the time is `GetTime()`'s
//! session clock, one value per frame, where ClassicAPI reads the tick count at the call; and the
//! slots live with the VM, so a reload while a bar runs reads them empty until the next start.

use mlua::{Lua, MultiValue, Value};

/// The three types the wire names, in its order: fatigue (`EXHAUSTION`), breath, feign death.
const NAMES: [&str; 3] = ["EXHAUSTION", "BREATH", "FEIGNDEATH"];

/// 3.3.5's `MirrorTimerInfo`; an empty slot is all zero and reads `"UNKNOWN"`.
#[derive(Clone, Default, PartialEq, Debug)]
struct Timer {
    active: bool,
    /// ms, as of `start`.
    value: i32,
    /// ms.
    max: i32,
    /// ms of value per ms of time: negative drains, positive refills.
    scale: i32,
    paused: bool,
    label: String,
    spell_id: u32,
    /// `GetTime()` at the start, in seconds.
    start: f64,
}

#[derive(Default)]
struct State([Timer; 3]);

/// One start as the packet carries it, with the bar's caption the app resolved for its event.
pub struct MirrorTimerStart<'a> {
    pub kind: u32,
    pub value: i32,
    pub max: i32,
    pub scale: i32,
    pub paused: bool,
    pub spell_id: u32,
    pub label: &'a str,
}

/// Record a start in its type's slot; a type past the three is dropped, as ClassicAPI drops it.
pub fn start(lua: &Lua, start: MirrorTimerStart) {
    let now = crate::script::clock::now(lua);
    let Some(mut state) = lua.app_data_mut::<State>() else {
        return;
    };
    let Some(slot) = state.0.get_mut(start.kind as usize) else {
        return;
    };
    *slot = Timer {
        active: true,
        value: start.value,
        max: start.max,
        scale: start.scale,
        paused: start.paused,
        label: start.label.to_string(),
        spell_id: start.spell_id,
        start: now,
    };
}

/// Empty `kind`'s slot.
pub fn stop(lua: &Lua, kind: u32) {
    if let Some(mut state) = lua.app_data_mut::<State>() {
        if let Some(slot) = state.0.get_mut(kind as usize) {
            *slot = Timer::default();
        }
    }
}

/// 3.3.5's `value + scale * (now - start)`, except that a paused timer holds its value, which
/// 3.3.5 keeps extrapolating; an empty slot reads 0. Whole milliseconds, as ClassicAPI's ints.
fn progress(timer: &Timer, now: f64) -> f64 {
    if timer.paused {
        return f64::from(timer.value);
    }
    let elapsed = ((now - timer.start) * 1000.0).round().max(0.0);
    f64::from(timer.value) + f64::from(timer.scale) * elapsed
}

/// A Lua number or a string that reads as one, as `lua_isnumber` takes it.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Number(n) => Some(*n),
        Value::String(s) => s.to_str().ok()?.trim().parse().ok(),
        _ => None,
    }
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(State::default());
    let g = lua.globals();
    // Six values as 3.3.5 returns them plus retail's spell id; an empty slot answers "UNKNOWN" and
    // zeros, an index outside 1 to 3 answers nothing.
    g.set(
        "GetMirrorTimerInfo",
        lua.create_function(|lua, index: Value| {
            let Some(index) = number(&index) else {
                return Err(mlua::Error::runtime("Usage: GetMirrorTimerInfo(index)"));
            };
            let index = index as i64 - 1;
            let state = lua
                .app_data_ref::<State>()
                .expect("mirror timers installed");
            let Some(timer) = usize::try_from(index).ok().and_then(|i| state.0.get(i)) else {
                return Ok(MultiValue::new());
            };
            let name = if timer.active {
                NAMES[index as usize]
            } else {
                "UNKNOWN"
            };
            Ok(MultiValue::from_vec(vec![
                Value::String(lua.create_string(name)?),
                Value::Number(f64::from(timer.value)),
                Value::Number(f64::from(timer.max)),
                Value::Number(f64::from(timer.scale)),
                Value::Number(if timer.paused { 1.0 } else { 0.0 }),
                Value::String(lua.create_string(&timer.label)?),
                Value::Number(f64::from(timer.spell_id)),
            ]))
        })?,
    )?;
    // The name matches case-insensitively; one that names no timer answers nil.
    g.set(
        "GetMirrorTimerProgress",
        lua.create_function(|lua, timer: Value| {
            let name = match &timer {
                Value::String(s) => s.to_string_lossy(),
                Value::Integer(_) | Value::Number(_) => timer.to_string()?,
                _ => {
                    return Err(mlua::Error::runtime(
                        "Usage: GetMirrorTimerProgress(\"timer\")",
                    ))
                }
            };
            let Some(kind) = NAMES.iter().position(|n| n.eq_ignore_ascii_case(&name)) else {
                return Ok(Value::Nil);
            };
            let now = crate::script::clock::now(lua);
            let state = lua
                .app_data_ref::<State>()
                .expect("mirror timers installed");
            Ok(Value::Number(progress(&state.0[kind], now)))
        })?,
    )?;
    Ok(())
}
