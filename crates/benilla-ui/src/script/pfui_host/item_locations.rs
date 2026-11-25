//! ClassicAPI's `C_Item` calls that take an item location (`item/Location.cpp` and `Exists.cpp`,
//! `Link.cpp`, `GUID.cpp`, `GetItemLocation.cpp`, `Lock.cpp`, `Data.cpp`, `MaxStackSize.cpp`,
//! `Info.cpp`), which ClassicAPI's `ItemMixin` and `ItemLocationMixin` build on: pfUI's bag code
//! reads its slots through `Item:CreateFromBagAndSlot`.
//!
//! A location is a table, `{equipmentSlotIndex = n}` (the 1-based inventory slot `GetInventoryItem*`
//! takes) or `{bagID = b, slotIndex = s}` (the container numbering `GetContainerItem*` takes, a
//! bank bag only while the banker is open), or an item GUID string as `GetItemGUID` writes it,
//! which names the item wherever the player holds it. Any other argument is no location: the
//! getters raise their usage error, `DoesItemExist` and `IsLocked` answer false.

use mlua::{Lua, MultiValue, Table, Value};

use crate::script::Model;

/// Inventory slots `GetInventoryItem*` and a location's `equipmentSlotIndex` take: the doll and
/// the equipped bags (`INVSLOT_*`, 1..=23).
const EQUIPMENT_SLOTS: std::ops::RangeInclusive<usize> = 1..=23;

/// Where a location found its item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Place {
    Equipment(usize),
    Bag(i64, u32),
}

/// The item a location names.
struct Found {
    place: Place,
    item_id: u32,
    /// The item object's guid, 0 while it has not streamed.
    guid: u64,
}

/// `IsLocationArg`: a table or a string.
fn is_location(value: &Value) -> bool {
    matches!(value, Value::Table(_) | Value::String(_))
}

/// A strict item GUID string: `0x` and 16 hex digits.
fn parse_guid(text: &str) -> Option<u64> {
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    (hex.len() == 16)
        .then(|| u64::from_str_radix(hex, 16).ok())
        .flatten()
        .filter(|guid| *guid != 0)
}

/// A table field read as the engine reads it: a number (or numeric string), truncated.
fn int_field(lua: &Lua, table: &Table, key: &str) -> Option<i64> {
    let value: Value = table.get(key).ok()?;
    let number = lua.coerce_number(value).ok().flatten()?;
    number.is_finite().then_some(number.trunc() as i64)
}

/// The item at an equipment slot, with its guid.
fn equipment(lua: &Lua, model: &Model, slot: usize) -> Option<Found> {
    if !EQUIPMENT_SLOTS.contains(&slot) {
        return None;
    }
    let view = crate::script::worn::worn_slot(lua, model, slot).filter(|v| v.item_id != 0)?;
    let guid = crate::script::worn::worn_keys(lua, model)
        .get(slot)
        .copied()
        .flatten()
        .map_or(0, |(guid, _)| guid);
    Some(Found {
        place: Place::Equipment(slot),
        item_id: view.item_id,
        guid,
    })
}

/// The item at a container slot, through the bank gates.
fn bag(lua: &Lua, model: &Model, bag: i64, slot: u32) -> Option<Found> {
    let held = crate::script::held::bag_slot(lua, model, bag, slot).filter(|s| s.item_id != 0)?;
    Some(Found {
        place: Place::Bag(bag, slot),
        item_id: held.item_id,
        guid: held.guid,
    })
}

/// The item of `guid`, worn or in a container, a bank one only while the banker is open.
fn by_guid(lua: &Lua, model: &Model, guid: u64) -> Option<Found> {
    let worn = crate::script::worn::worn_keys(lua, model);
    if let Some(slot) = EQUIPMENT_SLOTS.clone().find(|&slot| {
        worn.get(slot)
            .copied()
            .flatten()
            .is_some_and(|(g, _)| g == guid)
    }) {
        return equipment(lua, model, slot);
    }
    crate::script::held::bag_items(lua, model)
        .into_iter()
        .find(|(bag, _, item)| {
            item.guid == guid && !model.vault_shut(*bag) && !model.bank_bag_shut(*bag)
        })
        .map(|(bag, slot, item)| Found {
            place: Place::Bag(bag, slot),
            item_id: item.item_id,
            guid,
        })
}

/// `Item::Location::Resolve`: the item a location names, `None` for an empty or unknown place.
fn resolve(lua: &Lua, value: &Value) -> Option<Found> {
    let model = lua.app_data_ref::<Model>().expect("model");
    match value {
        Value::String(text) => by_guid(lua, &model, parse_guid(&text.to_string_lossy())?),
        Value::Table(table) => {
            if let Some(slot) = int_field(lua, table, "equipmentSlotIndex") {
                return equipment(lua, &model, usize::try_from(slot).ok()?);
            }
            let bag_id = int_field(lua, table, "bagID")?;
            let slot = u32::try_from(int_field(lua, table, "slotIndex")?).ok()?;
            bag(lua, &model, bag_id, slot)
        }
        _ => None,
    }
}

fn usage(call: &str) -> mlua::Error {
    mlua::Error::runtime(format!("Usage: C_Item.{call}(itemLocation)"))
}

/// The location's item, or the call's usage error for no location.
fn located(lua: &Lua, call: &str, value: &Value) -> mlua::Result<Option<Found>> {
    if !is_location(value) {
        return Err(usage(call));
    }
    Ok(resolve(lua, value))
}

/// The link the stock getter of the item's place builds: `GetContainerItemLink` or
/// `GetInventoryItemLink("player", slot)`, the engine's link builder ClassicAPI calls.
fn link(lua: &Lua, place: Place) -> mlua::Result<Value> {
    let globals = lua.globals();
    match place {
        Place::Bag(bag, slot) => globals
            .get::<mlua::Function>("GetContainerItemLink")?
            .call((bag, slot)),
        Place::Equipment(slot) => globals
            .get::<mlua::Function>("GetInventoryItemLink")?
            .call(("player", slot)),
    }
}

/// An `ItemLocationMixin` table, as ClassicAPI pushes one (`MixinTable::Push`): the mixin's
/// methods copied in, then the place's fields.
fn location_table(lua: &Lua, place: Place) -> mlua::Result<Table> {
    let table = lua.create_table()?;
    if let Ok(Value::Table(mixin)) = lua.globals().get::<Value>("ItemLocationMixin") {
        for pair in mixin.pairs::<Value, Value>() {
            let (key, value) = pair?;
            table.set(key, value)?;
        }
    }
    match place {
        Place::Equipment(slot) => table.set("equipmentSlotIndex", slot)?,
        Place::Bag(bag, slot) => {
            table.set("bagID", bag)?;
            table.set("slotIndex", slot)?;
        }
    }
    Ok(table)
}

/// The item record's icon, asking for a record not cached (`PushIconForItemID`).
fn icon(lua: &Lua, item_id: u32) -> mlua::Result<Value> {
    let icon = super::cached(lua, Some(item_id), true).and_then(|item| item.icon);
    super::string(lua, icon.as_deref())
}

pub(super) fn install(lua: &Lua, api: &Table) -> mlua::Result<()> {
    api.set(
        "DoesItemExist",
        lua.create_function(|lua, value: Value| Ok(resolve(lua, &value).is_some()))?,
    )?;
    api.set(
        "GetItemLink",
        lua.create_function(|lua, value: Value| {
            let Some(found) = located(lua, "GetItemLink", &value)? else {
                return Ok(Value::Nil);
            };
            link(lua, found.place)
        })?,
    )?;
    api.set(
        "GetItemGUID",
        lua.create_function(|lua, value: Value| {
            let found = located(lua, "GetItemGUID", &value)?.filter(|f| f.guid != 0);
            match found {
                Some(found) => Ok(Value::String(
                    lua.create_string(super::super::guid_events::guid_string(found.guid))?,
                )),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    // The place that holds the item of a GUID now, as an `ItemLocationMixin`; nil for a stale GUID
    // or one the player does not hold there.
    api.set(
        "GetItemLocation",
        lua.create_function(|lua, value: Value| {
            let Value::String(text) = value else {
                return Err(mlua::Error::runtime(
                    "Usage: C_Item.GetItemLocation(itemGUID)",
                ));
            };
            let place = parse_guid(&text.to_string_lossy()).and_then(|guid| {
                let model = lua.app_data_ref::<Model>().expect("model");
                by_guid(lua, &model, guid).map(|found| found.place)
            });
            match place {
                Some(place) => Ok(Value::Table(location_table(lua, place)?)),
                None => Ok(Value::Nil),
            }
        })?,
    )?;
    api.set(
        "GetItemIcon",
        lua.create_function(
            |lua, value: Value| match located(lua, "GetItemIcon", &value)? {
                Some(found) => icon(lua, found.item_id),
                None => Ok(Value::Nil),
            },
        )?,
    )?;
    api.set(
        "GetItemMaxStackSize",
        lua.create_function(|lua, value: Value| {
            let Some(found) = located(lua, "GetItemMaxStackSize", &value)? else {
                return Ok(Value::Nil);
            };
            Ok(super::cached(lua, Some(found.item_id), true)
                .map_or(Value::Nil, |item| Value::Integer(i64::from(item.stackable))))
        })?,
    )?;
    // The id inside an id, an `item:` string or a link, the record not needed; nothing otherwise.
    api.set(
        "GetItemIDForItemInfo",
        lua.create_function(|lua, value: Value| {
            Ok(match super::id(lua, value)? {
                Some(id) => MultiValue::from_vec(vec![Value::Integer(i64::from(id))]),
                None => MultiValue::new(),
            })
        })?,
    )?;
    api.set(
        "IsItemDataCached",
        lua.create_function(|lua, value: Value| {
            let found = if is_location(&value) {
                resolve(lua, &value)
            } else {
                None
            };
            let Some(found) = found else {
                return Err(mlua::Error::runtime(
                    "Usage: local isCached = C_Item.IsItemDataCached(itemLocation)",
                ));
            };
            Ok(super::cached(lua, Some(found.item_id), false).is_some())
        })?,
    )?;
    api.set(
        "IsLocked",
        lua.create_function(|lua, value: Value| {
            Ok(resolve(lua, &value).is_some_and(|found| {
                let model = lua.app_data_ref::<Model>().expect("model");
                crate::script::cursor::item_locked(&model, found.guid)
            }))
        })?,
    )?;
    // The engine's own `LockItem` (`0x4953e0`) and `UnlockItem` (`0x495420`) on the item's guid,
    // each with its `ITEM_LOCK_CHANGED`: a client-side mark no packet goes with.
    for (name, lock) in [("LockItem", true), ("UnlockItem", false)] {
        api.set(
            name,
            lua.create_function(move |lua, value: Value| {
                let guid = located(lua, name, &value)?.map_or(0, |found| found.guid);
                if guid != 0 {
                    crate::script::tick::verb(lua, |model| {
                        if lock {
                            crate::script::cursor::lock_item(model, guid);
                        } else {
                            crate::script::cursor::unlock_item(model, guid);
                        }
                    });
                }
                Ok(())
            })?,
        )?;
    }
    Ok(())
}

/// `GetItemIcon(itemID)`, ClassicAPI's global: the icon path for a numeric id only.
pub(super) fn install_global(lua: &Lua) -> mlua::Result<()> {
    lua.globals().set(
        "GetItemIcon",
        lua.create_function(|lua, value: Value| {
            let id = match value {
                Value::Integer(_) | Value::Number(_) => lua.coerce_number(value)?,
                _ => None,
            };
            let Some(id) = id.filter(|id| id.is_finite() && *id >= 1.0) else {
                return Err(mlua::Error::runtime("Usage: GetItemIcon(itemID)"));
            };
            icon(lua, id as u32)
        })?,
    )
}
