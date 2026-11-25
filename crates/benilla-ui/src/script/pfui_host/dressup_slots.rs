//! ClassicAPI's `DressUpModel:UndressSlot(slotIndex)` (`model/DressUp.cpp`): one inventory slot's
//! step of `Undress`, handed to the dressing room ([`crate::script::dressup`]) as the stock verbs
//! are. Slots 16 and 17 are the held lanes; a slot outside 1-19 does nothing. Installed with the
//! layer, so the stock DressUpModel keeps 1.12's methods.

use mlua::{Lua, Table, Value};

use crate::script::dressup::{submit, DressUpIntent, REG_DRESSUPMODEL_METHODS};
use crate::script::object::frame_handle_of;

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let methods: Table = lua.named_registry_value(REG_DRESSUPMODEL_METHODS)?;
    methods.set(
        "UndressSlot",
        lua.create_function(|lua, (this, slot): (Table, Value)| {
            frame_handle_of(lua, &this)?;
            let Some(slot) = lua.coerce_number(slot)? else {
                return Err(mlua::Error::runtime(
                    "Usage: DressUpModel:UndressSlot(slotIndex)",
                ));
            };
            if let Ok(slot @ 1..=19) = u8::try_from(slot as i64) {
                submit(lua, DressUpIntent::UndressSlot(slot))?;
            }
            Ok(())
        })?,
    )
}
