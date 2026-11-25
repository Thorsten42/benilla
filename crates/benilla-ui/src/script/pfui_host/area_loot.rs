//! The VM half of ClassicAPI's `C_Loot` (`loot/Nearby.cpp`, `Scan.cpp`, `Request.cpp`, `Unit.cpp`,
//! `UnitItem.cpp`): the corpse walk behind `LootAllCorpses` (the AoE Loot binding) and
//! `ScanNearbyLoot`, and the single-corpse `LootUnit` / `LootUnitItem`. `LootUnit` is the
//! right-click on the corpse ([`super::interact`]'s click), whose loot leg opens its window.
// Deviation: that right-click also selects the corpse, where ClassicAPI's sender (`0x5df130`)
// leaves the target alone.
//!
//! The host owns the walk (`benilla-app`'s `pfui::area_loot`): it feeds the lootable corpses in
//! interact range ([`set_nearby`]), whether a walk runs ([`set_in_progress`]) and the corpses the
//! last walk read ([`set_results`]), takes the calls' requests ([`take_requests`]) and fires
//! `LOOT_SCAN_COMPLETED` when a walk ends. A walk is silent: its windows never open the loot frame.

use mlua::{Lua, Table, Value};

/// What a call asks of the host's walk.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LootRequest {
    /// `ScanNearbyLoot`: read each corpse, take nothing.
    Scan,
    /// `LootAllCorpses([max])`: take the coin and every item of each corpse, at most `max` of them.
    LootAll { max: Option<u32> },
    /// `LootUnitItem(guid, itemID)`: take one item from the corpse.
    UnitItem { guid: u64, item_id: u32 },
}

/// One corpse a walk read: its coin in copper and its rows `(itemID, count, link)`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ScannedCorpse {
    pub guid: u64,
    pub coin: u32,
    pub items: Vec<(u32, u32, Option<String>)>,
}

#[derive(Default)]
struct AreaLoot {
    nearby: Vec<u64>,
    in_progress: bool,
    results: Vec<ScannedCorpse>,
    requests: Vec<LootRequest>,
}

/// The lootable corpses in interact range now.
pub fn set_nearby(lua: &Lua, guids: Vec<u64>) {
    if let Some(mut state) = lua.app_data_mut::<AreaLoot>() {
        state.nearby = guids;
    }
}

/// Whether a walk runs now.
pub fn set_in_progress(lua: &Lua, running: bool) {
    if let Some(mut state) = lua.app_data_mut::<AreaLoot>() {
        state.in_progress = running;
    }
}

/// The corpses the walk has read so far.
pub fn set_results(lua: &Lua, results: Vec<ScannedCorpse>) {
    if let Some(mut state) = lua.app_data_mut::<AreaLoot>() {
        state.results = results;
    }
}

/// The calls' requests since the last take, in call order.
pub fn take_requests(lua: &Lua) -> Vec<LootRequest> {
    lua.app_data_mut::<AreaLoot>()
        .map(|mut state| std::mem::take(&mut state.requests))
        .unwrap_or_default()
}

fn parse_guid(value: &Value) -> Option<u64> {
    let Value::String(text) = value else {
        return None;
    };
    let text = text.to_string_lossy();
    let hex = text
        .strip_prefix("0x")
        .or_else(|| text.strip_prefix("0X"))?;
    u64::from_str_radix(hex, 16).ok().filter(|g| *g != 0)
}

fn mounted(lua: &Lua) -> bool {
    super::units::player_raw(lua).is_some_and(|unit| unit.mount_display != 0)
}

/// Queue a walk: false while one runs or while mounted.
fn start_walk(lua: &Lua, request: LootRequest) -> bool {
    if mounted(lua) {
        return false;
    }
    let mut state = lua.app_data_mut::<AreaLoot>().expect("installed");
    if state.in_progress {
        return false;
    }
    state.in_progress = true;
    state.requests.push(request);
    true
}

fn push(lua: &Lua, request: LootRequest) {
    lua.app_data_mut::<AreaLoot>()
        .expect("installed")
        .requests
        .push(request);
}

/// The cached record's link, as `C_Item.GetItemInfo` builds it.
fn item_link(lua: &Lua, item_id: u32) -> Option<String> {
    let c_item: Table = lua.globals().get("C_Item").ok()?;
    let info: mlua::Function = c_item.get("GetItemInfo").ok()?;
    let values: mlua::MultiValue = info.call(item_id).ok()?;
    match values.get(1) {
        Some(Value::String(s)) => Some(s.to_string_lossy()),
        _ => None,
    }
}

fn guid_table(lua: &Lua, guid: u64) -> mlua::Result<Table> {
    let row = lua.create_table()?;
    row.set("guid", super::guid_events::guid_string(guid))?;
    Ok(row)
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(AreaLoot::default());
    let api = match lua.globals().get::<Value>("C_Loot")? {
        Value::Table(t) => t,
        _ => lua.create_table()?,
    };
    api.set(
        "GetNearbyLootableUnits",
        lua.create_function(|lua, ()| {
            let nearby = lua
                .app_data_ref::<AreaLoot>()
                .map(|s| s.nearby.clone())
                .unwrap_or_default();
            let out = lua.create_table()?;
            for (i, guid) in nearby.into_iter().enumerate() {
                out.set(i + 1, guid_table(lua, guid)?)?;
            }
            Ok(out)
        })?,
    )?;
    api.set(
        "LootUnit",
        lua.create_function(|lua, guid: Value| {
            let Some(guid) = parse_guid(&guid) else {
                return Err(mlua::Error::runtime("Usage: C_Loot.LootUnit(guid)"));
            };
            // The right-click on the corpse, whose loot leg opens its ordinary window.
            if !mounted(lua) {
                super::interact::click(lua, guid);
            }
            Ok(())
        })?,
    )?;
    api.set(
        "LootUnitItem",
        lua.create_function(|lua, (guid, item): (Value, Value)| {
            let (Some(guid), Some(item_id)) = (
                parse_guid(&guid),
                lua.coerce_integer(item)?
                    .and_then(|i| u32::try_from(i).ok()),
            ) else {
                return Err(mlua::Error::runtime(
                    "Usage: C_Loot.LootUnitItem(guid, itemID)",
                ));
            };
            if mounted(lua) {
                return Ok(false);
            }
            push(lua, LootRequest::UnitItem { guid, item_id });
            Ok(true)
        })?,
    )?;
    api.set(
        "ScanNearbyLoot",
        lua.create_function(|lua, ()| Ok(start_walk(lua, LootRequest::Scan)))?,
    )?;
    api.set(
        "LootAllCorpses",
        lua.create_function(|lua, max: Value| {
            let max = lua
                .coerce_integer(max)?
                .and_then(|m| u32::try_from(m).ok())
                .filter(|m| *m > 0);
            Ok(start_walk(lua, LootRequest::LootAll { max }))
        })?,
    )?;
    api.set(
        "IsScanInProgress",
        lua.create_function(|lua, ()| {
            Ok(lua
                .app_data_ref::<AreaLoot>()
                .is_some_and(|s| s.in_progress))
        })?,
    )?;
    api.set(
        "GetLastScanResults",
        lua.create_function(|lua, ()| {
            let results = lua
                .app_data_ref::<AreaLoot>()
                .map(|s| s.results.clone())
                .unwrap_or_default();
            let out = lua.create_table()?;
            for (i, corpse) in results.into_iter().enumerate() {
                let row = guid_table(lua, corpse.guid)?;
                row.set("coin", corpse.coin)?;
                let items = lua.create_table()?;
                for (j, (item_id, count, link)) in corpse.items.into_iter().enumerate() {
                    let item = lua.create_table()?;
                    item.set("itemID", item_id)?;
                    item.set("count", count)?;
                    // The engine's full hyperlink, omitted while the record is not cached.
                    let link = match link {
                        Some(link) => Some(link),
                        None => item_link(lua, item_id),
                    };
                    if let Some(link) = link {
                        item.set("link", link)?;
                    }
                    items.set(j + 1, item)?;
                }
                row.set("items", items)?;
                out.set(i + 1, row)?;
            }
            Ok(out)
        })?,
    )?;
    lua.globals().set("C_Loot", api)?;
    Ok(())
}
