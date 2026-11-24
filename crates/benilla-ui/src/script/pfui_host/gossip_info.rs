//! `C_GossipInfo` and the gossip counts, as ClassicAPI serves them over 1.12's gossip state
//! (`gossip/Info.cpp`): the open window's options and quest rows as 3.80.2's tables, and selectors
//! by option or quest id that hand the stock selectors the row's place.
//!
//! The rows are the gossip message's own (`0xbbbe90` options, `0xbb74c0` quests), which the host
//! answers at the call ([`GossipInfoQuery`]): the stock getters' [`super::super::GossipQuery`]
//! drops the wire option index, the icon number and the quest id these tables carry. A window is
//! live from its message until its close, and not while its text query is still out (the
//! engine's pending latch, which its own option selector tests); outside one every list is empty
//! and every selector does nothing.
//!
//! Deviations: ClassicAPI also serves a quest-greeting window (`SMSG_QUESTGIVER_QUEST_LIST`)
//! through the quest lists; here those lists stay empty for it. `questInfoID` is 0, ClassicAPI's
//! answer for a quest not in the cache, and the gossip points of interest are never set.

use mlua::{Lua, MultiValue, Table, Value};

use crate::script::live::{ask, LiveQuery};

/// The open gossip window's rows as the message wrote them, `None` with no live window.
pub struct GossipInfoQuery;

/// One live gossip window.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GossipInfo {
    /// `(wire index, icon, label)` per used slot, in slot order.
    pub options: Vec<(u32, u8, String)>,
    /// `(quest id, dialog-status icon, level, title)` per row, in packet order.
    pub quests: Vec<(u32, u32, u32, String)>,
}

impl LiveQuery for GossipInfoQuery {
    type Answer = Option<GossipInfo>;
}

fn live(lua: &Lua) -> Option<GossipInfo> {
    ask(lua, &GossipInfoQuery).flatten()
}

/// `IsActiveQuest` (`0x4e2430`'s filter): icons 3 and 4 are active, the rest available.
fn is_active(icon: u32) -> bool {
    icon == 3 || icon == 4
}

/// The quest log's grey test: the player is more than `GetQuestGreenRange()` levels above it.
fn is_trivial(lua: &Lua, level: u32) -> mlua::Result<bool> {
    let g = lua.globals();
    let player: i64 = g
        .get::<mlua::Function>("UnitLevel")?
        .call::<Option<i64>>("player")?
        .unwrap_or(0);
    let gap: i64 = g
        .get::<mlua::Function>("GetQuestGreenRange")?
        .call::<Option<i64>>(())?
        .unwrap_or(0);
    Ok(player - i64::from(level) > gap)
}

fn quest_list(lua: &Lua, active: bool) -> mlua::Result<Table> {
    let out = lua.create_table()?;
    let Some(info) = live(lua) else {
        return Ok(out);
    };
    for (id, icon, level, title) in info.quests.iter().filter(|q| is_active(q.1) == active) {
        let row = lua.create_table()?;
        row.set("title", title.as_str())?;
        row.set("questLevel", *level)?;
        row.set("isTrivial", is_trivial(lua, *level)?)?;
        row.set("frequency", 0)?;
        row.set("repeatable", false)?;
        // Only an active row carries it: icon 4 is the server's "ready to turn in".
        if active {
            row.set("isComplete", *icon == 4)?;
        }
        row.set("isLegendary", false)?;
        row.set("isIgnored", false)?;
        row.set("questID", *id)?;
        row.set("isImportant", false)?;
        row.set("isMeta", false)?;
        row.set("questInfoID", 0)?;
        out.raw_set(out.raw_len() + 1, row)?;
    }
    Ok(out)
}

fn count_quests(lua: &Lua, active: bool) -> usize {
    live(lua).map_or(0, |info| {
        info.quests
            .iter()
            .filter(|q| is_active(q.1) == active)
            .count()
    })
}

fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Integer(i) => Some(*i as f64),
        Value::Number(n) => Some(*n),
        Value::String(s) => s.to_str().ok()?.trim().parse().ok(),
        _ => None,
    }
}

/// Call the stock selector `name` with the row's 1-based place and the rest of the arguments.
fn select(lua: &Lua, name: &str, place: usize, code: Value) -> mlua::Result<()> {
    lua.globals()
        .get::<mlua::Function>(name)?
        .call::<()>((place, code))
}

fn select_quest(lua: &Lua, id: Value, active: bool, usage: &str) -> mlua::Result<()> {
    let Some(target) = number(&id) else {
        return Err(mlua::Error::runtime(usage));
    };
    let Some(info) = live(lua) else {
        return Ok(());
    };
    let place = info
        .quests
        .iter()
        .filter(|q| is_active(q.1) == active)
        .position(|q| f64::from(q.0) == target.trunc());
    if let Some(place) = place {
        let name = if active {
            "SelectGossipActiveQuest"
        } else {
            "SelectGossipAvailableQuest"
        };
        select(lua, name, place + 1, Value::Nil)?;
    }
    Ok(())
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    let ns = match g.get::<Value>("C_GossipInfo")? {
        Value::Table(t) => t,
        _ => {
            let t = lua.create_table()?;
            g.set("C_GossipInfo", t.clone())?;
            t
        }
    };
    // The greeting buffer the stock getter reads, kept past the close.
    ns.set(
        "GetText",
        lua.create_function(|lua, ()| {
            lua.globals()
                .get::<mlua::Function>("GetGossipText")?
                .call::<Value>(())
        })?,
    )?;
    ns.set(
        "GetOptions",
        lua.create_function(|lua, ()| {
            let out = lua.create_table()?;
            for (i, (index, icon, label)) in live(lua)
                .map(|info| info.options)
                .unwrap_or_default()
                .into_iter()
                .enumerate()
            {
                let row = lua.create_table()?;
                row.set("gossipOptionID", index)?;
                row.set("name", label)?;
                // The wire's icon type, not a file id.
                row.set("icon", icon)?;
                row.set("status", 0)?;
                row.set("flags", 0)?;
                row.set("selectOptionWhenOnlyOption", false)?;
                row.set("orderIndex", i + 1)?;
                out.raw_set(i + 1, row)?;
            }
            Ok(out)
        })?,
    )?;
    let num_options =
        lua.create_function(|lua, ()| Ok(live(lua).map_or(0, |info| info.options.len())))?;
    ns.set("GetNumOptions", num_options.clone())?;
    g.set("GetNumGossipOptions", num_options)?;
    ns.set(
        "GetAvailableQuests",
        lua.create_function(|lua, ()| quest_list(lua, false))?,
    )?;
    ns.set(
        "GetActiveQuests",
        lua.create_function(|lua, ()| quest_list(lua, true))?,
    )?;
    let available = lua.create_function(|lua, ()| Ok(count_quests(lua, false)))?;
    let active = lua.create_function(|lua, ()| Ok(count_quests(lua, true)))?;
    ns.set("GetNumAvailableQuests", available.clone())?;
    ns.set("GetNumActiveQuests", active.clone())?;
    g.set("GetNumGossipAvailableQuests", available)?;
    g.set("GetNumGossipActiveQuests", active)?;
    // By the option's wire index; a stale id or a closed window sends nothing. `text` is the code
    // a coded option asks for, `confirmed` answers a box 1.12 options do not have.
    ns.set(
        "SelectOption",
        lua.create_function(|lua, (id, text): (Value, Value)| {
            let Some(target) = number(&id) else {
                return Err(mlua::Error::runtime(
                    "Usage: C_GossipInfo.SelectOption(optionID [, text, confirmed])",
                ));
            };
            let Some(info) = live(lua) else {
                return Ok(());
            };
            if let Some(place) = info
                .options
                .iter()
                .position(|o| f64::from(o.0) == target.trunc())
            {
                select(lua, "SelectGossipOption", place + 1, text)?;
            }
            Ok(())
        })?,
    )?;
    ns.set(
        "SelectOptionByIndex",
        lua.create_function(|lua, (order, text): (Value, Value)| {
            let Some(order) = number(&order) else {
                return Err(mlua::Error::runtime(
                    "Usage: C_GossipInfo.SelectOptionByIndex(optionID [, text, confirmed])",
                ));
            };
            let Some(info) = live(lua) else {
                return Ok(());
            };
            let order = order.trunc();
            if order >= 1.0 && order <= info.options.len() as f64 {
                select(lua, "SelectGossipOption", order as usize, text)?;
            }
            Ok(())
        })?,
    )?;
    ns.set(
        "SelectAvailableQuest",
        lua.create_function(|lua, id: Value| {
            select_quest(
                lua,
                id,
                false,
                "Usage: C_GossipInfo.SelectAvailableQuest(questID)",
            )
        })?,
    )?;
    ns.set(
        "SelectActiveQuest",
        lua.create_function(|lua, id: Value| {
            select_quest(
                lua,
                id,
                true,
                "Usage: C_GossipInfo.SelectActiveQuest(questID)",
            )
        })?,
    )?;
    ns.set(
        "CloseGossip",
        lua.create_function(|lua, ()| {
            lua.globals()
                .get::<mlua::Function>("CloseGossip")?
                .call::<()>(())
        })?,
    )?;
    // No gossip point of interest is ever set here.
    ns.set(
        "GetPoiForUiMapID",
        lua.create_function(|_, _: MultiValue| Ok(Value::Nil))?,
    )?;
    ns.set(
        "GetPoiInfo",
        lua.create_function(|_, _: MultiValue| Ok(Value::Nil))?,
    )?;
    Ok(())
}
