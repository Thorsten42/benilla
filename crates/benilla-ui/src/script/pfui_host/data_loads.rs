//! ClassicAPI's explicit data loads for items and quests, the asks `AsyncCallbackSystem` (the
//! embedded addon's `Util/AsyncCallbackSystem.lua`) and the `Item`/`Quest` mixins drive.
//!
//! `C_Item.RequestLoadItemDataByID(item)` / `RequestLoadItemData(location)` (`item/Data.cpp`):
//! `ITEM_DATA_LOAD_RESULT(itemID, success)` for each call, at once inside the call when the record
//! is cached or the ID is 0 or less, else when the server's answer lands, once per call made
//! while it waited. `GET_ITEM_INFO_RECEIVED`, the implicit loads' event, stays as it is.
//!
//! `C_QuestLog.RequestLoadQuestByID(questID)` (`quest/Data.cpp`): `QUEST_DATA_LOAD_RESULT(questID,
//! true)` at once for a cached record, else when it lands, whatever the cache said.

use std::collections::HashMap;

use mlua::{Lua, Table, Value};

use crate::script::{Model, RecordAsk, ScriptValue};

#[derive(Default)]
struct Pending {
    /// Calls waiting on each item's record.
    items: HashMap<u32, u32>,
    /// Calls waiting on each quest's record.
    quests: HashMap<u32, u32>,
}

fn fire(lua: &Lua, event: &str, id: i64, success: Option<bool>) {
    super::fire_pfui_event(
        lua,
        event,
        vec![
            ScriptValue::Int(id),
            success.map_or(ScriptValue::Nil, ScriptValue::Bool),
        ],
    );
}

fn request_item(lua: &Lua, id: Option<i64>) -> mlua::Result<()> {
    let Some(id) = id else { return Ok(()) };
    let entry = u32::try_from(id).ok().filter(|id| *id > 0);
    let Some(entry) = entry else {
        fire(lua, "ITEM_DATA_LOAD_RESULT", id, Some(false));
        return Ok(());
    };
    let cached = lua
        .app_data_ref::<Model>()
        .expect("model")
        .item_templates
        .contains_key(&entry);
    if cached {
        fire(lua, "ITEM_DATA_LOAD_RESULT", id, Some(true));
        return Ok(());
    }
    *lua.app_data_mut::<Pending>()
        .expect("installed")
        .items
        .entry(entry)
        .or_default() += 1;
    let _ = crate::script::calls::ask(
        lua,
        RecordAsk::Item {
            entry,
            owner: 0,
            callback: crate::script::ItemCallback::Extension,
        },
    );
    Ok(())
}

/// The host's item answer: each call that waited on it gets its result.
pub(crate) fn item_landed(lua: &Lua, item_id: u32, success: bool) {
    let waiting = lua
        .app_data_mut::<Pending>()
        .and_then(|mut pending| pending.items.remove(&item_id))
        .unwrap_or(0);
    for _ in 0..waiting {
        fire(
            lua,
            "ITEM_DATA_LOAD_RESULT",
            i64::from(item_id),
            Some(success),
        );
    }
}

/// Quest records landed: each call that waited on one gets its result.
pub fn quests_landed(lua: &Lua, ids: &[u32]) {
    for &id in ids {
        let waiting = lua
            .app_data_mut::<Pending>()
            .and_then(|mut pending| pending.quests.remove(&id))
            .unwrap_or(0);
        for _ in 0..waiting {
            fire(lua, "QUEST_DATA_LOAD_RESULT", i64::from(id), Some(true));
        }
    }
}

fn namespace(lua: &Lua, name: &str) -> mlua::Result<Table> {
    let g = lua.globals();
    match g.get::<Value>(name)? {
        Value::Table(t) => Ok(t),
        _ => {
            let t = lua.create_table()?;
            g.set(name, t.clone())?;
            Ok(t)
        }
    }
}

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Pending::default());
    let item = namespace(lua, "C_Item")?;
    item.set(
        "RequestLoadItemDataByID",
        lua.create_function(|lua, value: Value| {
            let id = match &value {
                Value::Integer(_) | Value::Number(_) => lua.coerce_integer(value)?,
                _ => super::items::item_id(lua, value)?.map(i64::from),
            };
            request_item(lua, id)
        })?,
    )?;
    item.set(
        "RequestLoadItemData",
        lua.create_function(|lua, location: Value| {
            let id = super::items::location_item_id(lua, location)?.map(i64::from);
            request_item(lua, id)
        })?,
    )?;
    let quest = namespace(lua, "C_QuestLog")?;
    quest.set(
        "RequestLoadQuestByID",
        lua.create_function(|lua, id: Value| {
            let Some(id) = lua
                .coerce_integer(id)?
                .and_then(|id| u32::try_from(id).ok())
                .filter(|id| *id > 0)
            else {
                return Ok(());
            };
            let cached = lua
                .app_data_ref::<Model>()
                .expect("model")
                .quest_store
                .records
                .contains_key(&id);
            if cached {
                fire(lua, "QUEST_DATA_LOAD_RESULT", i64::from(id), Some(true));
                return Ok(());
            }
            *lua.app_data_mut::<Pending>()
                .expect("installed")
                .quests
                .entry(id)
                .or_default() += 1;
            let _ = crate::script::calls::ask(lua, RecordAsk::QuestRecord(id));
            Ok(())
        })?,
    )?;
    Ok(())
}
