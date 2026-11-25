//! ClassicAPI's `PlayerModel:SetDisplayInfo(displayID)` (`model/DisplayInfo.cpp`): the pane shows
//! a creature display, dressed as the creature, with no unit behind it. It is the subject the
//! stable's display-only paperdoll uses ([`crate::script::Model::set_pane_display`]); `SetUnit`
//! clears it again. Display 0 changes nothing. Installed by the layer, so a stock PlayerModel
//! keeps 1.12's surface.
// Deviation: ClassicAPI's `SetCreature(creatureID [, displayID])` is not built: it reads the
// creature cache's record, which the VM does not hold.

use mlua::{Lua, Table, Value};

use crate::script::modelframe::{with_model, REG_PLAYERMODEL_METHODS};
use crate::script::object::frame_handle_of;
use crate::script::Model;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let Some(methods) = lua.named_registry_value::<Option<Table>>(REG_PLAYERMODEL_METHODS)? else {
        return Ok(()); // Glue has no PlayerModel.
    };
    methods.set(
        "SetDisplayInfo",
        lua.create_function(|lua, (this, display): (Table, Value)| {
            with_model(lua, &this, |_| ())?;
            let Some(display) = lua.coerce_number(display)? else {
                return Err(mlua::Error::runtime("Usage: SetDisplayInfo(displayID)"));
            };
            let display = display as u32;
            if display == 0 {
                return Ok(());
            }
            let handle = frame_handle_of(lua, &this)?;
            lua.app_data_mut::<Model>()
                .expect("model app_data")
                .set_pane_display(handle, display);
            Ok(())
        })?,
    )
}
