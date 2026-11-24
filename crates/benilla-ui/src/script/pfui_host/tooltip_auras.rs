//! `GameTooltip:SetSpellByID` through the stock spell builder, and `SetUnitAura` through the stock
//! aura tooltips (`SetUnitBuff`, `SetUnitDebuff`), its `C_UnitAuras` index turned into the stock
//! one first.

use mlua::{Function, Lua, MultiValue, Table, Value};

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    let methods: Table = lua.named_registry_value(crate::script::tooltip::REG_TOOLTIP_METHODS)?;
    methods.set(
        "SetSpellByID",
        lua.create_function(|lua, (this, id): (Table, u32)| {
            super::set_spell_by_id(lua, &this, id, None, super::SpellRenderOpts::default())
        })?,
    )?;
    methods.set(
        "SetUnitAura",
        lua.create_function(
            |lua, (this, unit, index, filter): (Table, Value, Value, Option<String>)| {
                let harmful = filter.as_deref().is_some_and(|filter| {
                    filter
                        .split(['|', ' '])
                        .any(|token| token.eq_ignore_ascii_case("HARMFUL"))
                });
                let (index, harmful) = engine_index(lua, &unit, &index, harmful)
                    .map_or((index, harmful), |(engine, helpful)| {
                        (Value::Integer(engine), !helpful)
                    });
                let method = if harmful {
                    "SetUnitDebuff"
                } else {
                    "SetUnitBuff"
                };
                this.get::<Function>(method)?
                    .call::<MultiValue>((this.clone(), unit, index))
            },
        )?,
    )?;
    // The `ByAuraInstanceID` forms (ClassicAPI `unit/Tooltip.cpp`): the aura with that instance
    // ID, when the filter selects it, in the stock tooltip. `SetUnitAura…`'s filter defaults to
    // both ranges; the Buff and Debuff forms put their range in place of the filter's and keep
    // the rest. An ID the filter rejects, or one no aura has, leaves the tooltip as it was.
    for (name, forced) in [
        ("SetUnitAuraByAuraInstanceID", None),
        ("SetUnitBuffByAuraInstanceID", Some("HELPFUL")),
        ("SetUnitDebuffByAuraInstanceID", Some("HARMFUL")),
    ] {
        let usage = format!("Usage: GameTooltip:{name}(\"unit\", auraInstanceID [, \"filter\"])");
        methods.set(
            name,
            lua.create_function(
                move |lua, (this, unit, id, filter): (Table, Value, Value, Option<String>)| {
                    let (Value::String(token), Some(id)) = (&unit, lua.coerce_integer(id)?) else {
                        return Err(mlua::Error::runtime(usage.clone()));
                    };
                    let token = token.to_string_lossy();
                    let mut filter = filter.unwrap_or_default();
                    if let Some(range) = forced {
                        let rest: Vec<&str> = filter
                            .split(['|', ' '])
                            .filter(|t| {
                                !t.is_empty()
                                    && !t.eq_ignore_ascii_case("HELPFUL")
                                    && !t.eq_ignore_ascii_case("HARMFUL")
                            })
                            .collect();
                        filter = std::iter::once(range)
                            .chain(rest)
                            .collect::<Vec<_>>()
                            .join("|");
                    }
                    let Some(aura) = u32::try_from(id).ok().and_then(|id| {
                        crate::script::pfui_host::auras::by_instance(lua, &token, id)
                    }) else {
                        return Ok(MultiValue::new());
                    };
                    if !crate::script::pfui_host::auras::selects(lua, &token, &aura, &filter) {
                        return Ok(MultiValue::new());
                    }
                    let Some((index, helpful)) = stock_index(lua, &token, &aura) else {
                        return Ok(MultiValue::new());
                    };
                    let method = if helpful {
                        "SetUnitBuff"
                    } else {
                        "SetUnitDebuff"
                    };
                    this.get::<Function>(method)?.call::<MultiValue>((
                        this.clone(),
                        unit.clone(),
                        index,
                    ))
                },
            )?,
        )?;
    }
    Ok(())
}

/// The stock index of the aura `index` names in the `C_UnitAuras` order, and its polarity, as
/// ClassicAPI's `SetUnitAura` translates (`unit/Tooltip.cpp`: `FindNthSlot`, then
/// `EngineIndexForSlot`). The modern list puts the player's own auras first, so its index is not
/// the stock walk's; only the filter's HELPFUL/HARMFUL half is read. `None` passes the raw index
/// through, as ClassicAPI does when its list has nothing there.
fn engine_index(lua: &Lua, unit: &Value, index: &Value, harmful: bool) -> Option<(i64, bool)> {
    let token = lua.coerce_string(unit.clone()).ok()??.to_string_lossy();
    let nth = usize::try_from(lua.coerce_integer(index.clone()).ok()??)
        .ok()?
        .checked_sub(1)?;
    let polarity = if harmful { "HARMFUL" } else { "HELPFUL" };
    let aura = crate::script::pfui_host::auras::list(lua, &token, Some(polarity))
        .into_iter()
        .nth(nth)?;
    stock_index(lua, &token, &aura)
}

/// The stock index of `aura` on `token` and its polarity: its place among the unit's auras of its
/// polarity in ascending slot, as the stock setter counts them (`EngineIndexForSlot`).
fn stock_index(
    lua: &Lua,
    token: &str,
    aura: &crate::script::pfui_host::auras::Aura,
) -> Option<(i64, bool)> {
    let helpful = aura.state.helpful;
    let guid = crate::script::aura::aura_unit(lua, token).ok()?;
    // The stock setter counts the unit's auras of one polarity in ascending slot.
    crate::script::aura::read_auras_of(lua, guid, |list| {
        let mut count = 0;
        let mut found = None;
        list?.find(&mut |a| {
            if a.helpful() != helpful {
                return false;
            }
            count += 1;
            let state = a.state();
            let hit = state.slot == aura.slot && state.spell_id == aura.state.spell_id;
            if hit {
                found = Some(count);
            }
            hit
        });
        found
    })
    .map(|n| (n, helpful))
}
