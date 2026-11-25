//! pfUI's immutable spell lookups; gameplay-dependent queries live in the host bridge.

use crate::creature::{localized, number, numeric_id, rows, string};
use mlua::{Lua, Table, Value};
use std::{collections::BTreeMap, rc::Rc};

mod legacy;
#[path = "spell_runtime.rs"]
pub mod runtime;
mod states;

/// Effect columns the catalog keeps for the layer's own reads (IsDressableItem) that ClassicAPI's
/// `C_Spell.GetSpellEffectInfo` does not list (`spell/EffectInfo.cpp`): Spell.dbc's
/// `EffectItemType` and `EffectTriggerSpell`.
const INTERNAL_EFFECT_FIELDS: [&str; 2] = ["itemType", "triggerSpell"];
/// `SPELL_EFFECT_LEARN_SPELL` and `SPELL_EFFECT_CREATE_ITEM`.
const EFFECT_LEARN_SPELL: f64 = 36.0;
const EFFECT_CREATE_ITEM: f64 = 24.0;

#[derive(Clone, Debug)]
pub struct Spell {
    pub name: String,
    pub rank: String,
    pub icon: Option<String>,
    pub dispel: u32,
    pub attributes: u32,
    pub reagents: Vec<(u32, u32)>,
    pub effects: Vec<BTreeMap<&'static str, f64>>,
    pub cast_time: i32,
    pub min_range: f32,
    pub max_range: f32,
    pub cost: u32,
    pub power_type: i32,
    pub funnel: bool,
    /// How the spell moves the swing clock when it goes off.
    pub swing: Option<crate::swing::Kind>,
}

#[derive(Clone, Debug)]
pub struct Catalog(pub BTreeMap<u32, Spell>);

impl Catalog {
    pub fn read(
        read: &dyn Fn(&str) -> Option<Vec<u8>>,
        locale: &str,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let slots = [
            "enUS", "koKR", "frFR", "deDE", "zhCN", "zhTW", "esES", "esMX",
        ];
        let locale = slots.iter().position(|name| *name == locale).unwrap_or(0);
        let spells = rows(read, "Spell", 173)?;
        let icons = rows(read, "SpellIcon", 2)?;
        let cast_times = rows(read, "SpellCastTimes", 4)?;
        let ranges = rows(read, "SpellRange", 22)?;
        let mut times = BTreeMap::new();
        for row in cast_times.records() {
            times.insert(number(row, 0)?, (number(row, 1)? as i32).max(0));
        }
        let mut limits = BTreeMap::new();
        for row in ranges.records() {
            limits.insert(
                number(row, 0)?,
                (
                    f32::from_bits(number(row, 1)?),
                    f32::from_bits(number(row, 2)?),
                ),
            );
        }
        let mut paths = BTreeMap::new();
        for icon in icons.records() {
            paths.insert(number(icon, 0)?, string(&icons, icon, 1)?);
        }
        let mut result = BTreeMap::new();
        for row in spells.records() {
            let mut reagents = Vec::new();
            for i in 0..8 {
                let id = number(row, 42 + i)? as i32;
                if id > 0 {
                    reagents.push((id as u32, number(row, 50 + i)?));
                }
            }
            let mut effects = Vec::new();
            for i in 0..3 {
                let mut effect = BTreeMap::new();
                for (name, col, signed, float) in [
                    ("effect", 61, false, false),
                    ("auraName", 91, false, false),
                    ("basePoints", 76, true, false),
                    ("baseDice", 67, true, false),
                    ("dieSides", 64, true, false),
                    ("pointsPerLevel", 73, false, true),
                    ("dicePerLevel", 70, false, true),
                    ("miscValue", 106, true, false),
                    // Read by the layer itself, not in GetSpellEffectInfo ([`INTERNAL_EFFECT_FIELDS`]).
                    ("itemType", 103, false, false),
                    ("triggerSpell", 109, false, false),
                ] {
                    let value = number(row, col + i)?;
                    let value = if float {
                        f64::from(f32::from_bits(value))
                    } else if signed {
                        f64::from(value as i32)
                    } else {
                        f64::from(value)
                    };
                    effect.insert(name, value);
                }
                effects.push(effect);
            }
            result.insert(
                number(row, 0)?,
                Spell {
                    name: localized(&spells, row, 120, locale)?,
                    rank: localized(&spells, row, 129, locale)?,
                    icon: paths
                        .get(&number(row, 117)?)
                        .filter(|s| !s.is_empty())
                        .cloned(),
                    dispel: number(row, 4)?,
                    attributes: number(row, 6)?,
                    reagents,
                    effects,
                    cast_time: times.get(&number(row, 18)?).copied().unwrap_or(0),
                    min_range: limits.get(&number(row, 36)?).map_or(0.0, |range| range.0),
                    max_range: limits.get(&number(row, 36)?).map_or(0.0, |range| range.1),
                    cost: number(row, 32)?,
                    power_type: number(row, 31)? as i32,
                    funnel: number(row, 8)? & 0x800 != 0,
                    swing: crate::swing::Kind::of(
                        number(row, 6)?,
                        number(row, 8)?,
                        number(row, 21)?,
                    ),
                },
            );
        }
        Ok(Self(result))
    }

    pub fn install(&self, lua: &Lua) -> mlua::Result<()> {
        if !lua.globals().get::<Value>("C_Spell")?.is_nil() {
            return Err(mlua::Error::runtime("C_Spell already installed"));
        }
        let api = lua.create_table()?;
        crate::swing::install_kinds(
            lua,
            self.0
                .iter()
                .filter_map(|(id, spell)| Some((*id, spell.swing?))),
        );
        let records = Rc::new(self.0.clone());
        legacy::install(lua, records.clone())?;
        let r = records.clone();
        api.set(
            "GetSpellInfo",
            lua.create_function(move |lua, input: Value| {
                let Some((id, spell)) =
                    numeric_id(lua, input)?.and_then(|id| r.get(&id).map(|spell| (id, spell)))
                else {
                    return Ok(None);
                };
                let data = lua.create_table()?;
                data.set("name", spell.name.as_str())?;
                data.set("rank", spell.rank.as_str())?;
                data.set("spellID", id)?;
                data.set("iconID", spell.icon.as_deref())?;
                data.set("castTime", spell.cast_time)?;
                data.set("minRange", spell.min_range)?;
                data.set("maxRange", spell.max_range)?;
                data.set("cost", spell.cost)?;
                data.set("powerType", spell.power_type)?;
                data.set("isFunnel", spell.funnel)?;
                Ok(Some(data))
            })?,
        )?;
        let r = records.clone();
        let book = lua.create_table()?;
        book.set(
            "GetPlayerSpellsByAura",
            lua.create_function(move |lua, aura: i64| {
                let result = lua.create_table()?;
                if aura <= 0 {
                    return Ok(result);
                }
                for id in benilla_ui::script::pfui_host::units::known_spells(lua) {
                    if r.get(&id).is_some_and(|spell| {
                        spell.effects.iter().any(|effect| {
                            effect.get("auraName") == Some(&(aura as f64))
                                && effect.get("effect").is_some_and(|effect| *effect != 0.0)
                        })
                    }) {
                        result.push(id)?;
                    }
                }
                Ok(result)
            })?,
        )?;
        // ClassicAPI's `C_SpellBook.GetSpellBookItemInfo(slotIndex, spellBank)` (`spell/Info.cpp`):
        // the 1-based slot of the player's book (bank 0) or the pet's (1), nil for an empty or
        // out-of-range slot. Spell or PetAction, the spell id as the action, the rank as
        // `subName` ("" for none), the icon as its path (no file ids), never off-spec.
        book.set(
            "GetSpellBookItemInfo",
            lua.create_function(|lua, (slot, bank): (Value, Option<i64>)| {
                let Some(index) = lua
                    .coerce_integer(slot)?
                    .and_then(|slot| usize::try_from(slot).ok())
                    .and_then(|slot| slot.checked_sub(1))
                else {
                    return Ok(None);
                };
                let pet = bank == Some(1);
                let Some(view) = benilla_ui::script::pfui_host::book_slot(lua, pet, index) else {
                    return Ok(None);
                };
                let info = lua.create_table()?;
                info.set("itemType", if pet { 3 } else { 1 })?;
                info.set("actionID", view.spell_id)?;
                info.set("spellID", view.spell_id)?;
                info.set("name", view.name)?;
                info.set("subName", view.rank.unwrap_or_default())?;
                info.set("iconID", view.texture)?;
                info.set("isPassive", view.passive)?;
                info.set("isOffSpec", false)?;
                Ok(Some(info))
            })?,
        )?;
        lua.globals().set("C_SpellBook", book)?;
        for name in ["GetSpellName", "GetSpellSubtext", "GetSpellTexture"] {
            let records = records.clone();
            api.set(
                name,
                lua.create_function(move |lua, id: Value| {
                    let id = numeric_id(lua, id)?;
                    let spell = id.and_then(|id| records.get(&id));
                    Ok(spell
                        .and_then(|s| match name {
                            "GetSpellName" => Some(s.name.as_str()).filter(|s| !s.is_empty()),
                            "GetSpellSubtext" => Some(s.rank.as_str()),
                            _ => s.icon.as_deref(),
                        })
                        .map(str::to_owned))
                })?,
            )?;
        }
        let r = records.clone();
        api.set(
            "GetSpellDispelType",
            lua.create_function(move |lua, id: Value| {
                Ok(numeric_id(lua, id)?
                    .and_then(|id| r.get(&id))
                    .map_or(0, |s| s.dispel))
            })?,
        )?;
        let r = records.clone();
        api.set(
            "IsSpellPassive",
            lua.create_function(move |lua, id: Value| {
                Ok(numeric_id(lua, id)?
                    .and_then(|id| r.get(&id))
                    .is_some_and(|s| s.attributes & 0x40 != 0))
            })?,
        )?;
        let r = records.clone();
        api.set(
            "IsNextMeleeSpell",
            lua.create_function(move |lua, id: Value| {
                Ok(numeric_id(lua, id)?
                    .and_then(|id| r.get(&id))
                    .is_some_and(|s| s.attributes & 0x404 != 0))
            })?,
        )?;
        let r = records.clone();
        api.set(
            "GetSpellReagents",
            lua.create_function(move |lua, id: Value| {
                let Some(spell) = numeric_id(lua, id)?.and_then(|id| r.get(&id)) else {
                    return Ok(None);
                };
                let result = lua.create_table()?;
                for (index, (id, count)) in spell.reagents.iter().enumerate() {
                    let reagent = lua.create_table()?;
                    reagent.set("itemID", *id)?;
                    reagent.set("count", *count)?;
                    result.set(index + 1, reagent)?;
                }
                Ok(Some(result))
            })?,
        )?;
        states::install(lua, records.clone())?;
        let r = records.clone();
        api.set(
            "GetSpellEffectInfo",
            lua.create_function(move |lua, id: Value| {
                let Some(spell) = numeric_id(lua, id)?.and_then(|id| r.get(&id)) else {
                    return Ok(None);
                };
                let result = lua.create_table()?;
                for (index, fields) in spell.effects.iter().enumerate() {
                    let effect = lua.create_table()?;
                    for (name, value) in fields {
                        if !INTERNAL_EFFECT_FIELDS.contains(name) {
                            effect.set(*name, *value)?;
                        }
                    }
                    result.set(index + 1, effect)?;
                }
                Ok(Some(result))
            })?,
        )?;
        // ClassicAPI's spell loads (`spell/Data.cpp`): Spell.dbc is resident, so a spell is cached
        // exactly when its record exists and a load answers inside the call, `(spellID, true)` for
        // a record and `(spellID, nil)` for an id with none; an identifier that names no spell id
        // fires nothing.
        let r = records.clone();
        api.set(
            "IsSpellDataCached",
            lua.create_function(move |lua, spell: Value| {
                let id = benilla_ui::script::pfui_host::units::spell_identifier(lua, spell)?;
                Ok(id.is_some_and(|id| r.contains_key(&id)))
            })?,
        )?;
        // ClassicAPI's `C_Spell.DoesSpellExist(spellID)` (`spell/Exists.cpp`): a number naming a
        // `Spell.dbc` record; any other value, or an id below 1, is false.
        let r = records.clone();
        api.set(
            "DoesSpellExist",
            lua.create_function(move |lua, spell: Value| {
                let id = match spell {
                    Value::Integer(_) | Value::Number(_) => lua.coerce_number(spell)?,
                    Value::String(_) => lua.coerce_number(spell)?,
                    _ => None,
                };
                Ok(id
                    .filter(|id| id.is_finite() && *id >= 1.0 && *id <= f64::from(u32::MAX))
                    .is_some_and(|id| r.contains_key(&(id as u32))))
            })?,
        )?;
        let r = records.clone();
        api.set(
            "RequestLoadSpellData",
            lua.create_function(move |lua, spell: Value| {
                let id = match &spell {
                    Value::Integer(_) | Value::Number(_) => lua
                        .coerce_integer(spell)?
                        .and_then(|id| u32::try_from(id).ok()),
                    _ => benilla_ui::script::pfui_host::units::spell_identifier(lua, spell)?,
                };
                let Some(id) = id else { return Ok(()) };
                let found = r.contains_key(&id);
                benilla_ui::script::pfui_host::fire_pfui_event(
                    lua,
                    "SPELL_DATA_LOAD_RESULT",
                    vec![
                        benilla_ui::script::ScriptValue::Int(i64::from(id)),
                        if found {
                            benilla_ui::script::ScriptValue::Bool(true)
                        } else {
                            benilla_ui::script::ScriptValue::Nil
                        },
                    ],
                );
                Ok(())
            })?,
        )?;
        // ClassicAPI's `IsDressableItem(item)` (`item/Dressable.cpp`, 4.3.4's): 1 for an item with
        // an inventory type, or a recipe whose spell (effect 0 LEARN_SPELL) teaches one whose
        // effect 0 creates such an item; else nil. Read from the cache, so an uncached item or
        // product is not dressable. Deviation: the recipe is tested through its first use spell,
        // where ClassicAPI walks all five of the item's spells.
        let r = records.clone();
        lua.globals().set(
            "IsDressableItem",
            lua.create_function(move |lua, item: Value| {
                let globals = lua.globals();
                let c_item: Table = globals.get("C_Item")?;
                let inventory_type = |id: i64| -> mlua::Result<bool> {
                    Ok(c_item
                        .get::<mlua::Function>("GetItemInventoryTypeByID")?
                        .call::<Option<i64>>(id)?
                        .is_some_and(|t| t != 0))
                };
                let Some(id) = c_item
                    .get::<mlua::Function>("GetItemInfoInstant")?
                    .call::<Option<i64>>(item)?
                else {
                    return Ok(None);
                };
                if inventory_type(id)? {
                    return Ok(Some(1));
                }
                let (_, spell): (Value, Option<u32>) =
                    c_item.get::<mlua::Function>("GetItemSpell")?.call(id)?;
                let effect0 = |spell: u32| r.get(&spell).and_then(|s| s.effects.first().cloned());
                let created = spell
                    .and_then(effect0)
                    .filter(|e| e.get("effect") == Some(&EFFECT_LEARN_SPELL))
                    .and_then(|e| e.get("triggerSpell").map(|t| *t as u32))
                    .and_then(effect0)
                    .filter(|e| e.get("effect") == Some(&EFFECT_CREATE_ITEM))
                    .and_then(|e| e.get("itemType").map(|t| *t as i64))
                    .filter(|id| *id > 0);
                Ok(match created {
                    Some(made) if inventory_type(made)? => Some(1),
                    _ => None,
                })
            })?,
        )?;
        lua.globals().set("C_Spell", api)?;
        benilla_ui::script::pfui_host::auras::install_spell_cancel(lua)?;
        benilla_ui::script::pfui_host::macro_verbs::install_spell_verbs(lua)?;
        benilla_ui::script::pfui_host::casts::install(lua)?;
        lua.load(SPELL_ENGINE)
            .set_name("@benilla-pfui/spell_engine.lua")
            .exec()?;
        runtime::install(lua)
    }
}

/// The ClassicAPI calls built on the catalog's own `C_Spell`.
const SPELL_ENGINE: &str = r#"
function C_Spell.CastingInfo() return C_Spell.UnitCastingInfo("player") end

-- A single school's name; a mask of more than one school has none here.
function C_Spell.GetSchoolString(mask)
  mask = tonumber(mask) or 0
  local masks = {1, 2, 4, 8, 16, 32, 64}
  for school = 0, 6 do
    if mask == masks[school + 1] then return getglobal("SPELL_SCHOOL" .. school .. "_CAP") or "Unknown" end
  end
  return "Unknown"
end

-- Modern's link alone; the global `GetSpellLink` adds the id.
function C_Spell.GetSpellLink(spell) return (GetSpellLink(spell)) end
"#;
