//! ClassicAPI's `GetMouseButtonClicked()` and `GetRunningMacroButton()` (`macro/` and
//! `frame/ClickEvents.cpp`).
//!
//! `GetMouseButtonClicked` names the mouse button that started the handler running now:
//! `OnMouseDown`, `OnMouseUp`, `PreClick`, `OnClick`, `PostClick` or `OnDoubleClick`, on any frame,
//! and the helpers it calls; nil at any other time. The name is "LeftButton", "MiddleButton",
//! "RightButton", "Button4" or "Button5"; any other button argument reads "UNKNOWN". The handler
//! fire marks it around the call ([`enter`]), restoring an outer handler's.
//!
//! `GetRunningMacroButton` is the button a running macro was started with: nil for a macro an
//! action button runs.
// Deviation: ClassicAPI answers "LeftButton" for a macro a `SetBindingMacro` key runs; this client
// does not tell the macro run its start, so that macro reads nil too.

use mlua::{Lua, Value};

#[derive(Default)]
struct Clicked(Option<String>);

const HANDLERS: [&str; 6] = [
    "OnMouseDown",
    "OnMouseUp",
    "PreClick",
    "OnClick",
    "PostClick",
    "OnDoubleClick",
];

const BUTTONS: [&str; 5] = [
    "LeftButton",
    "MiddleButton",
    "RightButton",
    "Button4",
    "Button5",
];

/// Restores the outer handler's button when the handler's call ends.
pub(crate) struct Mark<'a> {
    lua: &'a Lua,
    outer: Option<Option<String>>,
}

impl Drop for Mark<'_> {
    fn drop(&mut self) {
        if let Some(outer) = self.outer.take() {
            if let Some(mut clicked) = self.lua.app_data_mut::<Clicked>() {
                clicked.0 = outer;
            }
        }
    }
}

/// Mark the button of a mouse or click handler about to run (`script`, its first argument), for
/// the length of the returned guard; any other handler marks nothing.
pub(crate) fn enter<'a>(lua: &'a Lua, script: &str, first: Option<&Value>) -> Mark<'a> {
    let mut mark = Mark { lua, outer: None };
    if !HANDLERS.contains(&script) {
        return mark;
    }
    let Some(mut clicked) = lua.app_data_mut::<Clicked>() else {
        return mark;
    };
    let name = match first {
        Some(Value::String(s)) => {
            let s = s.to_string_lossy();
            if BUTTONS.contains(&s.as_str()) {
                s
            } else {
                "UNKNOWN".to_string()
            }
        }
        _ => "LeftButton".to_string(),
    };
    mark.outer = Some(clicked.0.replace(name));
    mark
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Clicked::default());
    lua.globals().set(
        "GetMouseButtonClicked",
        lua.create_function(|lua, ()| {
            Ok(lua
                .app_data_ref::<Clicked>()
                .and_then(|clicked| clicked.0.clone()))
        })?,
    )?;
    lua.globals().set(
        "GetRunningMacroButton",
        lua.create_function(|_, ()| Ok(Value::Nil))?,
    )?;
    Ok(())
}
