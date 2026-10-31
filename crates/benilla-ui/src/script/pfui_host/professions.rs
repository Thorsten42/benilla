//! The open profession windows, as the extension's profession links (`|Htrade:…|h`) read them.

use crate::script::Model;
use mlua::Lua;

/// The open TradeSkill window: its `SkillLine.dbc` line and `GetTradeSkillLine`'s rank and
/// maximum, each with the permanent bonus.
pub fn open_trade_skill(lua: &Lua) -> Option<(u32, u32, u32)> {
    let model = lua.app_data_ref::<Model>()?;
    let state = model
        .trade_skill
        .as_ref()
        .filter(|_| model.trade_skill_open)?;
    Some((state.line, state.rank, state.max_rank))
}

/// The open Craft window: its craft type (the opener's `EffectMiscValue[0]`) and its rank and
/// maximum, each with the permanent bonus.
pub fn open_craft(lua: &Lua) -> Option<(u32, u32, u32)> {
    let model = lua.app_data_ref::<Model>()?;
    let state = model.craft.as_ref().filter(|_| model.craft_open)?;
    Some((state.craft_type, state.rank, state.max_rank))
}
