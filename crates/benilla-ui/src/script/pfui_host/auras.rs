//! The modern aura projection, with real descriptor slots kept separately from stock DTOs.

use crate::script::{AuraState, Model};
use mlua::{Lua, MultiValue, Table, Value};
use std::collections::HashMap;

#[derive(Clone, PartialEq)]
pub struct Aura {
    /// The actual zero-based UNIT_FIELD_AURA slot; never the display-list position.
    pub slot: u8,
    pub state: AuraState,
    /// Unknown until a corresponding cast was observed.
    pub source_guid: Option<u64>,
    /// DBC mechanics and the descriptor's active aura effects, supplied by the host.
    pub crowd_control: bool,
    /// Every effect of the spell targets its caster alone, so the unit wearing it cast it.
    pub self_only: bool,
    /// One of the spell's effects applies an aura, which a player who knows it could cast.
    pub applies_aura: bool,
    /// The spell's active immunity effects, which can keep an interrupt or silence off its wearer.
    pub immunities: Vec<super::interrupts::Immunity>,
}

/// An aura's group in the order the modern getters list them: the player's or their pet's, then
/// what the player could apply, then the rest. Slot order holds within each group.
fn aura_rank(aura: &Aura, wearer: u64, units: &crate::script::UnitGuids, known: bool) -> u8 {
    let caster = aura
        .source_guid
        .or(aura.self_only.then_some(wearer))
        .unwrap_or(0);
    if is_ours(caster, units) {
        0
    } else if aura.applies_aura && known {
        1
    } else {
        2
    }
}

fn is_ours(caster: u64, units: &crate::script::UnitGuids) -> bool {
    caster != 0 && (caster == units.player || caster == units.pet)
}

#[derive(Default)]
struct Extras {
    by_guid: HashMap<u64, Vec<Aura>>,
    /// Each player aura's last expiry by `(slot, spell)`, with the full duration it had when that
    /// expiry was first read.
    player_spans: HashMap<(u8, u32), (f64, f64)>,
    /// Each unit's aura instances by slot: the spell the slot held and the ID it was given
    /// (ClassicAPI's `aura/Instance.cpp`).
    instances: HashMap<u64, HashMap<u8, (u32, u32)>>,
    /// The last instance ID given out; IDs start at 1.
    last_instance: u32,
}

/// ClassicAPI's aura instance IDs: a slot's record follows its aura. A new spell in the slot ends
/// the old instance and starts one with a fresh ID; the same aura with new stacks or timing keeps
/// its ID; a unit whose list goes away is forgotten, and one seen again is given new IDs.
fn sync_instances(extras: &mut Extras, guid: u64, auras: Option<&[Aura]>) -> Delta {
    let Some(auras) = auras else {
        let removed = extras
            .instances
            .remove(&guid)
            .map(|old| old.values().map(|&(_, id)| id).collect())
            .unwrap_or_default();
        return Delta {
            removed,
            ..Delta::default()
        };
    };
    let first = !extras.instances.contains_key(&guid);
    let old = extras.instances.remove(&guid).unwrap_or_default();
    let previous = extras.by_guid.get(&guid);
    let mut delta = Delta {
        full: first,
        ..Delta::default()
    };
    let mut new = HashMap::with_capacity(auras.len());
    for aura in auras {
        let spell = aura.state.spell_id;
        let id = match old.get(&aura.slot) {
            Some(&(held, id)) if held == spell => {
                // The same instance: an update when its stacks or timing moved.
                let before = previous.and_then(|list| {
                    list.iter()
                        .find(|a| a.slot == aura.slot && a.state.spell_id == spell)
                });
                if before.is_some_and(|b| {
                    b.state.count != aura.state.count
                        || b.state.duration != aura.state.duration
                        || b.state.expiration_time != aura.state.expiration_time
                }) {
                    delta.updated.push(id);
                }
                id
            }
            _ => {
                extras.last_instance = extras.last_instance.wrapping_add(1).max(1);
                delta.added.push(extras.last_instance);
                extras.last_instance
            }
        };
        new.insert(aura.slot, (spell, id));
    }
    let kept: std::collections::HashSet<u32> = new.values().map(|&(_, id)| id).collect();
    delta.removed = old
        .values()
        .map(|&(_, id)| id)
        .filter(|id| !kept.contains(id))
        .collect();
    extras.instances.insert(guid, new);
    delta
}

/// One unit's aura change as ClassicAPI's `UNIT_AURA_DELTA` reports it (`aura/Delta.cpp`).
#[derive(Default)]
struct Delta {
    /// The unit was seen for the first time: its IDs are news.
    full: bool,
    added: Vec<u32>,
    updated: Vec<u32>,
    removed: Vec<u32>,
}

impl Delta {
    fn is_empty(&self) -> bool {
        !self.full && self.added.is_empty() && self.updated.is_empty() && self.removed.is_empty()
    }
}

/// ClassicAPI's `UNIT_AURA_DELTA(unitTarget, updateInfo)`, retail's UNIT_AURA payload under its own
/// name: once per token naming the unit, then to the frames that registered its GUID. The payload
/// carries instance IDs only (`isFullUpdate`, `addedAuraInstanceIDs`, `updatedAuraInstanceIDs`,
/// `removedAuraInstanceIDs`, each list nil when empty); a handler reads the auras by ID.
fn fire_delta(lua: &Lua, guid: u64, delta: &Delta) -> mlua::Result<()> {
    if delta.is_empty() {
        return Ok(());
    }
    let info = lua.create_table()?;
    info.set("isFullUpdate", delta.full)?;
    for (key, ids) in [
        ("addedAuraInstanceIDs", &delta.added),
        ("updatedAuraInstanceIDs", &delta.updated),
        ("removedAuraInstanceIDs", &delta.removed),
    ] {
        if !ids.is_empty() {
            info.set(key, lua.create_sequence_from(ids.iter().copied())?)?;
        }
    }
    for token in super::units::event_tokens(lua, guid) {
        let unit = Value::String(lua.create_string(token)?);
        super::fire_pfui_values(
            lua,
            "UNIT_AURA_DELTA",
            vec![unit, Value::Table(info.clone())],
        );
    }
    super::guid_events::signal_values(lua, "UNIT_AURA_DELTA", guid, vec![Value::Table(info)]);
    Ok(())
}

/// The instance ID of the aura in `slot` on `guid`, while it holds `spell`.
fn instance_id(lua: &Lua, guid: u64, slot: u8, spell: u32) -> Option<u32> {
    let extras = lua.app_data_ref::<Extras>()?;
    let &(held, id) = extras.instances.get(&guid)?.get(&slot)?;
    (held == spell).then_some(id)
}

/// The aura on `token` whose instance ID is `id`, with the player's timing read as the getters
/// read it.
pub(crate) fn by_instance(lua: &Lua, token: &str, id: u32) -> Option<Aura> {
    let guid = unit_guid(lua, token)?;
    let slot = {
        let extras = lua.app_data_ref::<Extras>()?;
        extras
            .instances
            .get(&guid)?
            .iter()
            .find(|(_, &(_, held))| held == id)
            .map(|(&slot, _)| slot)?
    };
    self::slot(lua, token, i64::from(slot))
}

/// Whether `filter` selects `aura` on `token`, by the list's own rules; a filter naming neither
/// HELPFUL nor HARMFUL accepts both.
pub(crate) fn selects(lua: &Lua, token: &str, aura: &Aura, filter: &str) -> bool {
    list(lua, token, Some(filter))
        .iter()
        .any(|a| a.slot == aura.slot && a.state.spell_id == aura.state.spell_id)
}

/// A player aura's timing as the getters answer it. The stock player list carries none: the
/// expiry is the duration packet's stamp, read through the lent host as `GetPlayerBuffTimeLeft`
/// reads it ([`crate::script::PlayerBuffExpiry`]), and the packet carries only the time left, so
/// the full duration is what was left when a new expiry was first read (the application's packet,
/// unless the first read came later). An observed cast's duration stays.
fn player_timing(lua: &Lua, mut aura: Aura) -> Aura {
    let expiry =
        crate::script::live::ask(lua, &crate::script::PlayerBuffExpiry { slot: aura.slot })
            .unwrap_or(aura.state.expiration_time);
    if expiry <= 0.0 {
        return aura;
    }
    let now = lua
        .globals()
        .get::<mlua::Function>("GetTime")
        .and_then(|time| time.call::<f64>(()))
        .unwrap_or(0.0);
    let key = (aura.slot, aura.state.spell_id);
    let duration = {
        let Some(mut extras) = lua.app_data_mut::<Extras>() else {
            return aura;
        };
        match extras.player_spans.get(&key) {
            Some(&(seen, duration)) if (seen - expiry).abs() < 0.01 => duration,
            _ => {
                let duration = (expiry - now).max(0.0);
                extras.player_spans.insert(key, (expiry, duration));
                duration
            }
        }
    };
    aura.state.expiration_time = expiry;
    if aura.state.duration <= 0.0 {
        aura.state.duration = duration;
    }
    aura
}

/// The stock aura feed owns roster fallback even after a member or pet leaves object range.
pub fn has_group_alias(lua: &Lua, guid: u64) -> bool {
    lua.app_data_ref::<Model>().is_some_and(|model| {
        let units = &model.unit_guids;
        units
            .party
            .iter()
            .chain(&units.party_pets)
            .chain(&units.raid)
            .chain(&units.raid_pets)
            .any(|candidate| *candidate == guid && guid != 0)
    })
}

/// Enrich a streamed aura only after an observed successful cast names its source.
pub fn annotate(lua: &Lua, guid: u64, spell: u32, caster: u64, duration: f64, expires: f64) {
    let player = lua
        .app_data_ref::<Model>()
        .is_some_and(|m| m.unit_guids.player == guid);
    let mut changed = false;
    {
        let Some(mut extras) = lua.app_data_mut::<Extras>() else {
            return;
        };
        if let Some(auras) = extras.by_guid.get_mut(&guid) {
            for aura in auras.iter_mut().filter(|a| a.state.spell_id == spell) {
                let before = aura.clone();
                aura.source_guid = Some(caster);
                // A refresh updates remote timing; a player's duration packet stays authoritative.
                if !player || aura.state.duration == 0.0 {
                    aura.state.duration = duration;
                    aura.state.expiration_time = expires;
                }
                changed |= *aura != before;
            }
        }
    }
    if changed {
        for token in super::units::event_tokens(lua, guid) {
            crate::script::pfui_host::fire_pfui_event(
                lua,
                "UNIT_AURA",
                vec![crate::script::ScriptValue::Str(token)],
            );
        }
    }
}

/// A raw removal/reuse ends that application's ownership even if no modern feed ran between.
pub fn forget_annotation(lua: &Lua, guid: u64, slot: u8, spell: u32) {
    let player = lua
        .app_data_ref::<Model>()
        .is_some_and(|m| m.unit_guids.player == guid);
    if let Some(mut extras) = lua.app_data_mut::<Extras>() {
        if let Some(aura) = extras.by_guid.get_mut(&guid).and_then(|list| {
            list.iter_mut()
                .find(|aura| aura.slot == slot && aura.state.spell_id == spell)
        }) {
            aura.source_guid = None;
            if !player {
                aura.state.duration = 0.0;
                aura.state.expiration_time = 0.0;
            }
        }
    }
}

/// Feed before UNIT_AURA dispatch. The host supplies raw slots, including out-of-range roster slots.
pub fn set_auras(lua: &Lua, guid: u64, mut auras: Option<Vec<Aura>>) {
    let player = lua
        .app_data_ref::<Model>()
        .is_some_and(|m| m.unit_guids.player == guid);
    if let (Some(incoming), Some(extras)) = (&mut auras, lua.app_data_ref::<Extras>()) {
        for aura in incoming {
            if let Some(old) = extras.by_guid.get(&guid).and_then(|list| {
                list.iter()
                    .find(|old| old.slot == aura.slot && old.state.spell_id == aura.state.spell_id)
            }) {
                aura.source_guid = aura.source_guid.or(old.source_guid);
                if !player && aura.state.duration == 0.0 {
                    aura.state.duration = old.state.duration;
                    aura.state.expiration_time = old.state.expiration_time;
                }
                // Joining the two clocks can introduce sub-millisecond rounding drift.
                if (aura.state.duration - old.state.duration).abs() < 0.001 {
                    aura.state.duration = old.state.duration;
                }
                if (aura.state.expiration_time - old.state.expiration_time).abs() < 0.001 {
                    aura.state.expiration_time = old.state.expiration_time;
                }
            }
        }
    }
    // Legacy UnitBuff/UnitDebuff on GUID/focus aliases read this same live list.
    if let Some(mut model) = lua.app_data_mut::<Model>() {
        if guid != model.unit_guids.player {
            match &auras {
                Some(auras) => {
                    let list: Vec<_> = auras.iter().map(|a| a.state.clone()).collect();
                    model.unit_auras.insert(guid, list.into());
                }
                None => {
                    model.unit_auras.remove(&guid);
                }
            }
        }
    }
    let (changed, timing_changed, delta) = if let Some(mut extras) = lua.app_data_mut::<Extras>() {
        let changed = extras.by_guid.get(&guid) != auras.as_ref();
        let timing_changed = player
            && auras
                .as_ref()
                .zip(extras.by_guid.get(&guid))
                .is_some_and(|(incoming, previous)| {
                    incoming.iter().any(|aura| {
                        previous.iter().any(|old| {
                            old.slot == aura.slot
                                && old.state.spell_id == aura.state.spell_id
                                && (old.state.duration != aura.state.duration
                                    || old.state.expiration_time != aura.state.expiration_time)
                        })
                    })
                });
        let delta = sync_instances(&mut extras, guid, auras.as_deref());
        if let Some(auras) = auras {
            extras.by_guid.insert(guid, auras);
        } else {
            extras.by_guid.remove(&guid);
        }
        (changed, timing_changed, delta)
    } else {
        (false, false, Delta::default())
    };
    if changed {
        // Stock PLAYER_AURAS_CHANGED deliberately ignores countdown-only refreshes.
        let tokens = if timing_changed {
            super::units::event_tokens(lua, guid)
        } else {
            super::units::custom_tokens(lua, guid)
        };
        for token in tokens {
            crate::script::pfui_host::fire_pfui_event(
                lua,
                "UNIT_AURA",
                vec![crate::script::ScriptValue::Str(token)],
            );
        }
        // A unit no token names still reaches the frames that registered its GUID.
        crate::script::pfui_host::guid_events::signal(lua, "UNIT_AURA", guid, &[]);
    }
    let _ = fire_delta(lua, guid, &delta);
}

pub(crate) fn unit_guid(lua: &Lua, token: &str) -> Option<u64> {
    if let Some(guid) = super::units::custom_guid(lua, token) {
        return guid;
    }
    if let Some(hex) = token
        .strip_prefix("0x")
        .or_else(|| token.strip_prefix("0X"))
    {
        return u64::from_str_radix(hex, 16).ok().filter(|guid| *guid != 0);
    }
    let model = lua.app_data_ref::<Model>()?;
    crate::script::unit::guid_of(lua, &model, token)
        .ok()
        .flatten()
}

/// The auras a unit wears, by GUID.
pub(crate) fn of_guid(lua: &Lua, guid: u64) -> Vec<Aura> {
    lua.app_data_ref::<Extras>()
        .and_then(|extras| extras.by_guid.get(&guid).cloned())
        .unwrap_or_default()
}

pub(crate) fn list(lua: &Lua, token: &str, filter: Option<&str>) -> Vec<Aura> {
    let Some(guid) = unit_guid(lua, token) else {
        return Vec::new();
    };
    let Some(extras) = lua.app_data_ref::<Extras>() else {
        return Vec::new();
    };
    let Some(auras) = extras.by_guid.get(&guid) else {
        return Vec::new();
    };
    let model = lua.app_data_ref::<Model>().expect("model");
    let player = guid == model.unit_guids.player;
    let tokens: Vec<_> = filter
        .unwrap_or("HELPFUL")
        .split(['|', ' '])
        .filter(|s| !s.is_empty())
        .map(str::to_ascii_uppercase)
        .collect();
    let has = |token: &str| tokens.iter().any(|s| s == token);
    let harmful = has("HARMFUL") || !has("HELPFUL");
    let helpful = has("HELPFUL") || !has("HARMFUL");
    let mut result: Vec<_> = auras
        .iter()
        .filter(|aura| {
            let state = &aura.state;
            if (state.helpful && (!helpful || has("!HELPFUL")))
                || (!state.helpful && (!harmful || has("!HARMFUL")))
            {
                return false;
            }
            let ours = aura_rank(aura, guid, &model.unit_guids, false) == 0;
            if (has("PLAYER") && !ours) || (has("!PLAYER") && ours) {
                return false;
            }
            let dispellable = state
                .debuff_type
                .as_deref()
                .is_some_and(|ty| matches!(ty, "Magic" | "Curse" | "Disease" | "Poison"));
            !((has("DISPELLABLE") && !dispellable)
                || (has("!DISPELLABLE") && dispellable)
                || (has("CROWD_CONTROL") && !aura.crowd_control)
                || (has("!CROWD_CONTROL") && aura.crowd_control))
        })
        .cloned()
        .collect();
    // Each polarity by group, then descriptor order, ordinary harmful slots before overflow slots.
    result.sort_by_cached_key(|aura| {
        let known = super::units::knows_spell(lua, aura.state.spell_id);
        (
            !aura.state.helpful,
            aura_rank(aura, guid, &model.unit_guids, known),
            !aura.state.helpful && aura.slot < 32,
            aura.slot,
        )
    });
    drop((model, extras));
    if player {
        result = result
            .into_iter()
            .map(|aura| player_timing(lua, aura))
            .collect();
    }
    result
}

fn slot(lua: &Lua, token: &str, slot: i64) -> Option<Aura> {
    let guid = unit_guid(lua, token)?;
    let aura = lua
        .app_data_ref::<Extras>()?
        .by_guid
        .get(&guid)?
        .iter()
        .find(|aura| i64::from(aura.slot) == slot)
        .cloned()?;
    let player = lua
        .app_data_ref::<Model>()
        .is_some_and(|model| model.unit_guids.player == guid);
    Some(if player {
        player_timing(lua, aura)
    } else {
        aura
    })
}

fn source(lua: &Lua, aura: &Aura) -> Option<String> {
    super::units::token_from_guid(lua, aura.source_guid?)
}

fn cast_by_player(aura: &Aura) -> bool {
    aura.source_guid.is_some_and(|guid| {
        // 1.12 player GUIDs have a zero high word; permanent pets use HIGHGUID_PET.
        guid != 0 && (guid >> 48 == 0 || guid >> 48 == 0xf140)
    })
}

/// The instance ID of `aura` as `unit` wears it.
fn aura_instance(lua: &Lua, unit: &str, aura: &Aura) -> Option<u32> {
    instance_id(lua, unit_guid(lua, unit)?, aura.slot, aura.state.spell_id)
}

fn data(lua: &Lua, unit: &str, aura: Option<Aura>) -> mlua::Result<Option<Table>> {
    let Some(aura) = aura else { return Ok(None) };
    let state = &aura.state;
    let table = lua.create_table()?;
    table.set("auraInstanceID", aura_instance(lua, unit, &aura))?;
    table.set("name", state.name.as_deref())?;
    table.set("icon", state.icon.as_deref())?;
    table.set("applications", state.count)?;
    table.set("spellId", state.spell_id)?;
    table.set("dispelName", state.debuff_type.as_deref().unwrap_or(""))?;
    table.set("duration", state.duration)?;
    table.set("expirationTime", state.expiration_time)?;
    table.set("sourceUnit", source(lua, &aura))?;
    table.set(
        "sourceGUID",
        aura.source_guid.map(|guid| format!("0x{guid:016X}")),
    )?;
    table.set("isHelpful", state.helpful)?;
    table.set("isHarmful", !state.helpful)?;
    table.set("isFromPlayerOrPlayerPet", cast_by_player(&aura))?;
    for name in [
        "isStealable",
        "isBossAura",
        "isNameplateOnly",
        "nameplateShowAll",
        "nameplateShowPersonal",
        "canApplyAura",
        "shouldConsolidate",
        "isRaid",
    ] {
        table.set(name, false)?;
    }
    table.set("charges", 0)?;
    table.set("maxCharges", 0)?;
    table.set("timeMod", 1)?;
    Ok(Some(table))
}

fn tuple(lua: &Lua, unit: &str, aura: Option<Aura>) -> mlua::Result<MultiValue> {
    let Some(aura) = aura else {
        return Ok(MultiValue::from_vec(vec![Value::Nil]));
    };
    let id = aura_instance(lua, unit, &aura);
    let string = |text: Option<&str>| -> mlua::Result<Value> {
        match text {
            Some(text) => Ok(Value::String(lua.create_string(text)?)),
            None => Ok(Value::Nil),
        }
    };
    let state = &aura.state;
    Ok(MultiValue::from_vec(vec![
        string(state.name.as_deref())?,
        string(state.icon.as_deref())?,
        Value::Integer(i64::from(state.count)),
        string(state.debuff_type.as_deref())?,
        Value::Number(state.duration),
        Value::Number(state.expiration_time),
        string(source(lua, &aura).as_deref())?,
        Value::Boolean(false),
        Value::Boolean(false),
        Value::Integer(i64::from(state.spell_id)),
        Value::Boolean(false),
        Value::Boolean(false),
        Value::Boolean(cast_by_player(&aura)),
        Value::Boolean(false),
        Value::Number(1.0),
        // ClassicAPI's 16th value: the aura's instance ID, nil where the table has none.
        id.map_or(Value::Nil, |id| Value::Integer(i64::from(id))),
    ]))
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Extras::default());
    let api = lua.create_table()?;
    api.set(
        "GetAuraDataByIndex",
        lua.create_function(
            |lua, (unit, index, filter): (String, i64, Option<String>)| {
                let aura = index
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| list(lua, &unit, filter.as_deref()).get(index).cloned());
                data(lua, &unit, aura)
            },
        )?,
    )?;
    api.set(
        "UnitAura",
        lua.create_function(
            |lua, (unit, index, filter): (String, i64, Option<String>)| {
                let aura = index
                    .checked_sub(1)
                    .and_then(|index| usize::try_from(index).ok())
                    .and_then(|index| list(lua, &unit, filter.as_deref()).get(index).cloned());
                tuple(lua, &unit, aura)
            },
        )?,
    )?;
    api.set(
        "GetAuraDataBySlot",
        lua.create_function(|lua, (unit, id): (String, Option<i64>)| {
            data(lua, &unit, id.and_then(|id| slot(lua, &unit, id)))
        })?,
    )?;
    api.set(
        "UnitAuraBySlot",
        lua.create_function(|lua, (unit, id): (String, Option<i64>)| {
            tuple(lua, &unit, id.and_then(|id| slot(lua, &unit, id)))
        })?,
    )?;
    api.set(
        "GetAuraDataBySpellName",
        lua.create_function(
            |lua, (unit, name, filter): (String, String, Option<String>)| {
                // The spell-name lookup searches both signs unless the caller supplies a filter.
                let aura = list(
                    lua,
                    &unit,
                    Some(filter.as_deref().unwrap_or("HELPFUL|HARMFUL")),
                )
                .into_iter()
                .find(|aura| aura.state.name.as_deref() == Some(&name));
                data(lua, &unit, aura)
            },
        )?,
    )?;
    api.set(
        "GetAuraSlots",
        lua.create_function(
            |lua,
             (unit, filter, max, continuation, output): (
                String,
                Option<String>,
                Option<u32>,
                Option<usize>,
                Option<Table>,
            )| {
                let auras = list(lua, &unit, filter.as_deref());
                let start = continuation.unwrap_or(0).min(auras.len());
                let count = max
                    .filter(|max| *max > 0)
                    .map_or(auras.len(), |max| max as usize);
                let end = start.saturating_add(count).min(auras.len());
                let next = if end < auras.len() {
                    Value::Integer(end as i64)
                } else {
                    Value::Nil
                };
                let batch = &auras[start..end];
                if let Some(output) = output {
                    let previous = output
                        .raw_get::<Option<usize>>("n")?
                        .unwrap_or(output.raw_len())
                        .min(48);
                    for (index, aura) in batch.iter().enumerate() {
                        output.raw_set(index + 1, aura.slot)?;
                    }
                    for index in batch.len() + 1..=previous {
                        output.raw_set(index, Value::Nil)?;
                    }
                    output.raw_set("n", batch.len())?;
                    return Ok(MultiValue::from_vec(vec![
                        next,
                        Value::Integer(batch.len() as i64),
                    ]));
                }
                let mut values = vec![next];
                values.extend(
                    batch
                        .iter()
                        .map(|aura| Value::Integer(i64::from(aura.slot))),
                );
                Ok(MultiValue::from_vec(values))
            },
        )?,
    )?;
    // The aura with that instance ID, or nil; with `buf`, its fields go into that table (cleared
    // of any this aura lacks), which is returned.
    api.set(
        "GetAuraDataByAuraInstanceID",
        lua.create_function(
            |lua, (unit, id, buf): (String, Option<u32>, Option<Table>)| {
                let aura = id.and_then(|id| by_instance(lua, &unit, id));
                let Some(table) = data(lua, &unit, aura)? else {
                    return Ok(Value::Nil);
                };
                let Some(buf) = buf else {
                    return Ok(Value::Table(table));
                };
                let keys: Vec<Value> = buf
                    .pairs::<Value, Value>()
                    .filter_map(|pair| pair.ok().map(|(key, _)| key))
                    .collect();
                for key in keys {
                    buf.raw_set(key, Value::Nil)?;
                }
                for pair in table.pairs::<Value, Value>() {
                    let (key, value) = pair?;
                    buf.raw_set(key, value)?;
                }
                Ok(Value::Table(buf))
            },
        )?,
    )?;
    // True when no aura on the unit has the ID, or the filter does not select it.
    api.set(
        "IsAuraFilteredOutByInstanceID",
        lua.create_function(
            |lua, (unit, id, filter): (String, Option<u32>, Option<String>)| {
                Ok(id
                    .and_then(|id| by_instance(lua, &unit, id))
                    .is_none_or(|aura| {
                        !selects(lua, &unit, &aura, filter.as_deref().unwrap_or(""))
                    }))
            },
        )?,
    )?;
    api.set(
        "GetAuraDispelTypeColor",
        lua.create_function(|lua, name: Option<String>| {
            let (r, g, b) = match name.as_deref() {
                Some("Magic") => (0.2, 0.6, 1.0),
                Some("Curse") => (0.6, 0.0, 1.0),
                Some("Disease") => (0.6, 0.4, 0.0),
                Some("Poison") => (0.0, 0.6, 0.0),
                Some("Enrage") => (1.0, 0.55, 0.0),
                _ => (0.8, 0.0, 0.0),
            };
            lua.globals()
                .get::<mlua::Function>("CreateColor")?
                .call::<Table>((r, g, b, 1.0))
        })?,
    )?;
    lua.globals().set("C_UnitAuras", api)
}

pub fn install_spell_cancel(lua: &Lua) -> mlua::Result<()> {
    let api: Table = lua.globals().get("C_Spell")?;
    api.set(
        "CancelSpellByID",
        lua.create_function(|lua, spell: u32| cancel_player_spell(lua, spell))?,
    )
}

/// Extension helpers use the same ownership and cancelability gate as the stock buff button, and
/// its cancel ([`crate::script::aura::cancel_aura`]).
pub fn cancel_player_spell(lua: &Lua, spell: u32) -> mlua::Result<()> {
    let authorized = lua
        .app_data_ref::<Model>()
        .expect("model")
        .player_auras
        .iter()
        .any(|aura| aura.spell_id == spell && crate::script::aura::cancel_authorized(aura));
    if authorized {
        crate::script::aura::cancel_aura(lua, spell)?;
    }
    Ok(())
}
