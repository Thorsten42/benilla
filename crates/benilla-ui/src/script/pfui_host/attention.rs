//! ClassicAPI's `FlashClientIcon([briefly])` (`screen/Flash.cpp`): the taskbar flash that reaches
//! an alt-tabbed player. The call only counts an ask; the app answers it on its window with the
//! window manager's one portable attention request (the taskbar flash on Windows, the urgency
//! hint on X11 and Wayland), which a focused window ignores, as the reference's flash does.
//! `briefly` picks between two Windows flash counts and has nothing to pick between here.

use mlua::{Lua, Value};

#[derive(Default)]
struct Asks(u32);

/// Whether the VM asked for the player's attention since the last call.
pub fn take(lua: &Lua) -> bool {
    lua.app_data_mut::<Asks>()
        .is_some_and(|mut asks| std::mem::take(&mut asks.0) > 0)
}

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Asks::default());
    lua.globals().set(
        "FlashClientIcon",
        lua.create_function(|lua, _briefly: Value| {
            if let Some(mut asks) = lua.app_data_mut::<Asks>() {
                asks.0 = asks.0.saturating_add(1);
            }
            Ok(())
        })?,
    )
}
