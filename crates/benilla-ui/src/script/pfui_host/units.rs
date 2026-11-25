//! Additional unit metadata belongs to the extension, separate from the stock token resolver.

use crate::script::{Model, UnitState};
use mlua::{Lua, Value};
use std::collections::{BTreeSet, HashMap};

#[path = "focus.rs"]
mod focus;
pub use focus::FocusMemory;

#[derive(Default)]
struct Metadata {
    creatures: HashMap<u64, u32>,
    traits: HashMap<u64, Traits>,
    known_spells: BTreeSet<u32>,
    powers: HashMap<u64, Power>,
    states: HashMap<u64, UnitState>,
    targets: HashMap<u64, u64>,
    focus: Option<u64>,
    restored_focus_pending: bool,
    raw: HashMap<u64, RawUnit>,
    /// The active player's `MOVEMENTFLAGS`, for `IsSwimming`.
    player_move_flags: u32,
    /// The units last reported in view, `None` until the first feed seeds it.
    in_view: Option<std::collections::HashSet<u64>>,
    /// The units of `boss1`..`boss5`, in order ([`focus::resolve`]).
    bosses: Vec<u64>,
    /// The creature IDs addons named bosses (`C_InstanceEncounter`), in the order added.
    boss_creatures: Vec<u32>,
}

/// Attach before installing the extension in a fresh VM. Only the owning session shares it.
pub fn retain_focus(lua: &Lua, memory: FocusMemory) {
    lua.set_app_data(memory);
}

pub fn clear_focus(lua: &Lua) {
    if lua.app_data_ref::<Metadata>().is_some() {
        focus::focus(lua, None);
    }
}

/// Announce a restored focus after the owning interface and its live snapshot are ready.
pub fn announce_restored_focus(lua: &Lua) {
    focus::announce_restore(lua);
}

/// Values from the engine's creature-type resolver and UNIT_CREATED_BY_SPELL descriptor.
pub struct Traits {
    pub creature_type: u32,
    pub created_by_spell: u32,
}

pub fn set_traits(lua: &Lua, units: impl IntoIterator<Item = (u64, Traits)>) {
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        metadata.traits = units.into_iter().collect();
    }
}

#[derive(Clone)]
pub struct RawUnit {
    pub health: Option<u32>,
    pub form: u8,
    pub mount_display: u32,
    /// `UNIT_FIELD_BYTES_2` byte 0, the sheath state: 0 put away, 1 melee, 2 ranged.
    pub sheath: u8,
    pub auras: Vec<u32>,
    pub applications: Vec<u8>,
}

/// The player's real descriptor snapshot for extension-owned form, mount and stealth helpers.
/// The active player's guid, 0 before the world entry.
pub fn player_guid(lua: &Lua) -> u64 {
    lua.app_data_ref::<Model>()
        .map_or(0, |model| model.unit_guids.player)
}

pub fn player_raw(lua: &Lua) -> Option<RawUnit> {
    let player = lua.app_data_ref::<Model>()?.unit_guids.player;
    lua.app_data_ref::<Metadata>()?.raw.get(&player).cloned()
}

pub fn player_is_warrior(lua: &Lua) -> bool {
    lua.app_data_ref::<Model>()
        .and_then(|model| {
            crate::script::unit::view(lua, &model, "player")
                .and_then(|unit| unit.class_file.clone())
        })
        .is_some_and(|class| class == "WARRIOR")
}

/// Every unit in view, as the object table holds it now. A unit that entered or left since the
/// last feed fires ClassicAPI's `UNIT_VISIBILITY_CHANGED: unitTarget, isVisible`
/// (`unit/VisibilityEvent.cpp`): once per token naming it, then to the frames that registered its
/// GUID. The first feed of a VM only seeds what is in view.
/// The units the feed reported whose name `matches`, for a search by name.
pub(crate) fn named_units(lua: &Lua, matches: impl Fn(&str) -> bool) -> Vec<u64> {
    lua.app_data_ref::<Metadata>()
        .map(|metadata| {
            metadata
                .states
                .iter()
                .filter(|(_, state)| state.exists && state.name.as_deref().is_some_and(&matches))
                .map(|(guid, _)| *guid)
                .collect()
        })
        .unwrap_or_default()
}

/// The active player's `MOVEMENTFLAGS` now, which `IsSwimming` reads.
pub fn set_player_move_flags(lua: &Lua, flags: u32) {
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        metadata.player_move_flags = flags;
    }
}

pub fn set_raw_units(lua: &Lua, units: impl IntoIterator<Item = (u64, RawUnit)>) {
    let changed: Vec<(u64, bool)> = {
        let Some(mut metadata) = lua.app_data_mut::<Metadata>() else {
            return;
        };
        metadata.raw = units.into_iter().collect();
        let now: std::collections::HashSet<u64> = metadata.raw.keys().copied().collect();
        let changed = match &metadata.in_view {
            None => Vec::new(),
            Some(before) => {
                let mut changed: Vec<(u64, bool)> = now
                    .difference(before)
                    .map(|guid| (*guid, true))
                    .chain(before.difference(&now).map(|guid| (*guid, false)))
                    .collect();
                changed.sort_unstable();
                changed
            }
        };
        metadata.in_view = Some(now);
        changed
    };
    for (guid, visible) in changed {
        let rest = [crate::script::ScriptValue::Bool(visible)];
        for token in event_tokens(lua, guid) {
            let mut args = vec![crate::script::ScriptValue::Str(token)];
            args.extend(rest.iter().cloned());
            crate::script::pfui_host::fire_pfui_event(lua, "UNIT_VISIBILITY_CHANGED", args);
        }
        crate::script::pfui_host::guid_events::signal(lua, "UNIT_VISIBILITY_CHANGED", guid, &rest);
    }
}

pub fn set_unit_states(lua: &Lua, states: impl IntoIterator<Item = (u64, UnitState, u64)>) {
    focus::feed_states(lua, states);
}

/// The unit `TargetByName` would find by `name`: the start of a unit's name, all of it with
/// `exact`, any case. A full match wins over a partial one; among equals, the lowest guid.
pub(super) fn guid_by_name(lua: &Lua, name: &str, exact: bool) -> Option<u64> {
    let wanted = name.to_lowercase();
    let metadata = lua.app_data_ref::<Metadata>()?;
    metadata
        .states
        .iter()
        .filter(|(_, state)| state.has_object)
        .filter_map(|(guid, state)| {
            let have = state.name.as_deref()?.to_lowercase();
            if have == wanted {
                Some((false, *guid))
            } else if !exact && have.starts_with(&wanted) {
                Some((true, *guid))
            } else {
                None
            }
        })
        .min()
        .map(|(_, guid)| guid)
}

/// The name the client holds for the unit `guid`: a nearby unit's, else a token's.
pub fn name_of(lua: &Lua, guid: u64) -> Option<String> {
    let held = lua
        .app_data_ref::<Metadata>()
        .and_then(|metadata| metadata.states.get(&guid)?.name.clone());
    held.or_else(|| {
        let token = token_from_guid(lua, guid)?;
        let model = lua.app_data_ref::<Model>()?;
        crate::script::unit::view(lua, &model, &token).and_then(|unit| unit.name.clone())
    })
}

pub fn custom_guid(lua: &Lua, token: &str) -> Option<Option<u64>> {
    focus::resolve(lua, token)
}

/// The complete token resolver, including the stock error for an unknown token.
pub fn guid(lua: &Lua, token: &str) -> mlua::Result<Option<u64>> {
    if let Some(guid) = custom_guid(lua, token) {
        return Ok(guid);
    }
    let model = lua.app_data_ref::<Model>().expect("model");
    crate::script::unit::guid_of(lua, &model, token)
}

/// Temporarily expose a committed macro selection to synchronous Lua unit queries. The normal
/// frame feed has not run again yet; all aliases unwind even when an addon macro runner errors.
pub(super) fn with_macro_target<R>(
    lua: &Lua,
    guid: Option<u64>,
    run: impl FnOnce() -> mlua::Result<R>,
) -> mlua::Result<R> {
    struct Scope<'a> {
        lua: &'a Lua,
        guids: crate::script::UnitGuids,
        target: Option<UnitState>,
        states: Vec<(u64, Option<UnitState>)>,
    }
    impl Drop for Scope<'_> {
        fn drop(&mut self) {
            let mut model = self.lua.app_data_mut::<Model>().expect("model");
            model.unit_guids = std::mem::take(&mut self.guids);
            if let Some(target) = self.target.take() {
                model.units_by_lower.insert("target".into(), target);
            } else {
                model.units_by_lower.remove("target");
            }
            for (guid, state) in self.states.drain(..) {
                if let Some(state) = state {
                    model.units_by_guid.insert(guid, state);
                } else {
                    model.units_by_guid.remove(&guid);
                }
            }
        }
    }
    let state = guid.and_then(|guid| focus::state(lua, guid));
    let mut scope = {
        let mut model = lua.app_data_mut::<Model>().expect("model");
        let scope = Scope {
            lua,
            guids: model.unit_guids.clone(),
            target: model.units_by_lower.remove("target"),
            states: Vec::new(),
        };
        model.unit_guids.target = guid.unwrap_or(0);
        if let Some(state) = state {
            model.units_by_lower.insert("target".into(), state);
        }
        scope
    };
    let mut current = guid;
    let mut seen = BTreeSet::new();
    while let Some(guid) = current.filter(|guid| *guid != 0) {
        if !seen.insert(guid) {
            break;
        }
        let state = focus::state(lua, guid);
        let next = lua
            .app_data_ref::<Metadata>()
            .and_then(|metadata| metadata.targets.get(&guid).copied())
            .or_else(|| scope.guids.held.get(&guid).copied())
            .unwrap_or(0);
        let mut model = lua.app_data_mut::<Model>().expect("model");
        model.unit_guids.held.insert(guid, next);
        if let Some(state) = state {
            scope
                .states
                .push((guid, model.units_by_guid.insert(guid, state)));
        }
        current = Some(next);
    }
    run()
}

pub(super) fn call(
    lua: &Lua,
    original: &mlua::Function,
    args: mlua::MultiValue,
) -> mlua::Result<mlua::MultiValue> {
    focus::call(lua, original, args)
}

/// The ClassicAPI reverse-lookup order, re-evaluated against the current token snapshots.
pub fn token_from_guid(lua: &Lua, wanted: u64) -> Option<String> {
    if wanted == 0 {
        return None;
    }
    let mut tokens = vec!["player".into(), "pet".into()];
    for (prefix, count) in [("party", 4), ("partypet", 4), ("raid", 40), ("raidpet", 40)] {
        tokens.extend((1..=count).map(|index| format!("{prefix}{index}")));
    }
    tokens.extend(super::super::nameplate::pfui::all_tokens(lua));
    tokens.extend(["target", "focus", "npc", "mouseover"].map(String::from));
    tokens
        .into_iter()
        .find(|token| guid(lua, token).ok().flatten() == Some(wanted))
}

/// Modern spell identifiers still choose name/rank through the stock player spellbook.
pub fn spell_identifier(lua: &Lua, input: Value) -> mlua::Result<Option<u32>> {
    if let Some(number) = lua.coerce_number(input.clone())? {
        return Ok(
            (number.is_finite() && number >= 1.0 && number <= f64::from(u32::MAX))
                .then_some(number as u32),
        );
    }
    let Value::String(input) = input else {
        return Ok(None);
    };
    let text = input.to_string_lossy();
    if let Some(link) = text
        .strip_prefix("spell:")
        .or_else(|| text.split_once("|Hspell:").map(|(_, tail)| tail))
    {
        let id = link.bytes().take_while(u8::is_ascii_digit).count();
        return Ok(link[..id].parse::<u32>().ok().filter(|id| *id != 0));
    }
    Ok(super::spell_named(lua, &text)
        .filter(|spell| !spell.pet)
        .map(|spell| spell.spell_id))
}

/// ClassicAPI's legacy metadata overload accepts player/pet slots and exact learned names.
pub fn legacy_spell_identifier(
    lua: &Lua,
    input: Value,
    book: Option<String>,
) -> mlua::Result<Option<u32>> {
    if let Some(book) = book {
        let Some(slot) = lua.coerce_number(input)?.filter(|n| {
            n.is_finite() && n.fract() == 0.0 && *n >= 1.0 && *n <= f64::from(u32::MAX)
        }) else {
            return Ok(None);
        };
        let slots = if book.eq_ignore_ascii_case("spell") {
            super::player_book(lua).slots
        } else if book.eq_ignore_ascii_case("pet") {
            super::pet_book(lua).slots
        } else {
            return Ok(None);
        };
        return Ok(slots.get(slot as usize - 1).map(|slot| slot.spell_id));
    }
    if let Some(number) = lua.coerce_number(input.clone())? {
        return Ok((number.is_finite()
            && number.fract() == 0.0
            && (1.0..=f64::from(u32::MAX)).contains(&number))
        .then_some(number as u32));
    }
    let Value::String(text) = input else {
        return Ok(None);
    };
    let text = text.to_string_lossy();
    if text.starts_with("spell:") || text.contains("|Hspell:") {
        return spell_identifier(lua, Value::String(lua.create_string(&text)?));
    }
    let query = text.trim();
    let (name, rank) = match (query.find('('), query.rfind(')')) {
        (Some(open), Some(close)) if close > open => {
            (query[..open].trim(), Some(query[open + 1..close].trim()))
        }
        _ => (query, None),
    };
    let (book, pets) = (super::player_book(lua), super::pet_book(lua));
    for slots in [&book.slots, &pets.slots] {
        if let Some(slot) = slots
            .iter()
            .filter(|slot| {
                slot.name == name && rank.is_none_or(|rank| slot.rank.as_deref() == Some(rank))
            })
            .enumerate()
            .max_by_key(|(index, slot)| {
                let rank = slot
                    .rank
                    .as_deref()
                    .unwrap_or("")
                    .split(|c: char| !c.is_ascii_digit())
                    .find(|text| !text.is_empty())
                    .and_then(|text| text.parse::<u32>().ok())
                    .unwrap_or(0);
                (rank, *index)
            })
            .map(|(_, slot)| slot)
        {
            return Ok(Some(slot.spell_id));
        }
    }
    Ok(None)
}

pub(super) fn custom_tokens(lua: &Lua, guid: u64) -> Vec<String> {
    query_tokens(lua)
        .into_iter()
        .filter(|token| custom_guid(lua, token) == Some(Some(guid)))
        .collect()
}

/// Current readable aliases for GUID-backed extension events; raw GUIDs are query-only.
pub(super) fn event_tokens(lua: &Lua, guid: u64) -> Vec<String> {
    let mut tokens = {
        let model = lua.app_data_ref::<Model>().expect("model");
        let mut tokens: Vec<String> = ["player", "pet", "target", "mouseover", "npc"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        tokens.extend((1..=4).flat_map(|i| [format!("party{i}"), format!("partypet{i}")]));
        tokens.extend(
            (1..=model.unit_guids.raid.len())
                .flat_map(|i| [format!("raid{i}"), format!("raidpet{i}")]),
        );
        tokens.retain(|token| {
            crate::script::unit::guid_of(lua, &model, token)
                .ok()
                .flatten()
                == Some(guid)
        });
        tokens
    };
    tokens.extend(custom_tokens(lua, guid));
    tokens
}

fn query_tokens(lua: &Lua) -> Vec<String> {
    let mut bases = super::super::nameplate::pfui::all_tokens(lua);
    bases.push("focus".into());
    bases.extend((1..=5).map(|index| format!("boss{index}")));
    bases.extend((1..=4).map(|index| format!("totem{index}")));
    bases.extend((1..=8).map(|index| format!("mark{index}")));
    bases
        .into_iter()
        .flat_map(|token| [format!("{token}target"), token])
        .collect()
}

#[derive(Clone, PartialEq)]
pub struct Power {
    pub kind: u8,
    pub current: [Option<u32>; 5],
    pub maximum: [Option<u32>; 5],
}

pub fn set_powers(lua: &Lua, powers: impl IntoIterator<Item = (u64, Power)>) {
    let powers: HashMap<_, _> = powers.into_iter().collect();
    let mut updates = Vec::new();
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        for (guid, power) in &powers {
            if let Some(previous) = metadata.powers.get(guid) {
                if previous.kind != power.kind {
                    updates.push((*guid, "UNIT_DISPLAYPOWER", None));
                }
                for (index, (current, maximum, kind)) in [
                    ("UNIT_MANA", "UNIT_MAXMANA", "MANA"),
                    ("UNIT_RAGE", "UNIT_MAXRAGE", "RAGE"),
                    ("UNIT_FOCUS", "UNIT_MAXFOCUS", "FOCUS"),
                    ("UNIT_ENERGY", "UNIT_MAXENERGY", "ENERGY"),
                    ("UNIT_HAPPINESS", "UNIT_MAXHAPPINESS", "HAPPINESS"),
                ]
                .into_iter()
                .enumerate()
                {
                    if previous.current[index] != power.current[index] {
                        updates.push((*guid, current, None));
                        updates.push((*guid, "UNIT_POWER_UPDATE", Some(kind)));
                    }
                    if previous.maximum[index] != power.maximum[index] {
                        updates.push((*guid, maximum, None));
                        updates.push((*guid, "UNIT_MAXPOWER", Some(kind)));
                    }
                }
            }
        }
        metadata.powers = powers;
    }
    for (guid, event, kind) in updates {
        let rest: Vec<_> = kind
            .map(|kind| crate::script::ScriptValue::Str(kind.into()))
            .into_iter()
            .collect();
        // The stock engine raises its own power events for the stock tokens; ClassicAPI's generic
        // ones (UNIT_POWER_UPDATE, UNIT_MAXPOWER) exist only here, so every token takes those.
        let tokens = if matches!(event, "UNIT_POWER_UPDATE" | "UNIT_MAXPOWER") {
            event_tokens(lua, guid)
        } else {
            custom_tokens(lua, guid)
        };
        for token in tokens {
            let mut args = vec![crate::script::ScriptValue::Str(token)];
            args.extend(rest.iter().cloned());
            crate::script::pfui_host::fire_pfui_event(lua, event, args);
        }
        // A unit no token names still reaches the frames that registered its GUID.
        crate::script::pfui_host::guid_events::signal(lua, event, guid, &rest);
    }
}

pub fn set_creature_ids(lua: &Lua, entries: impl IntoIterator<Item = (u64, u32)>) {
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        metadata.creatures = entries
            .into_iter()
            .filter(|(guid, id)| *guid != 0 && *id != 0)
            .collect();
    }
}

pub fn set_known_spells(lua: &Lua, spells: impl IntoIterator<Item = u32>) {
    if let Some(mut metadata) = lua.app_data_mut::<Metadata>() {
        metadata.known_spells = spells.into_iter().collect();
    }
}

pub fn knows_spell(lua: &Lua, spell: u32) -> bool {
    lua.app_data_ref::<Metadata>()
        .is_some_and(|metadata| metadata.known_spells.contains(&spell))
}

pub fn known_spells(lua: &Lua) -> Vec<u32> {
    lua.app_data_ref::<Metadata>()
        .map(|metadata| metadata.known_spells.iter().copied().collect())
        .unwrap_or_default()
}

pub fn creature_id(lua: &Lua, value: Value) -> mlua::Result<Option<u32>> {
    let Value::String(text) = value else {
        return Ok(None);
    };
    let text = text.to_string_lossy();
    let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) else {
        return Ok(None);
    };
    if hex.len() != 16 {
        return Ok(None);
    }
    let Ok(guid) = u64::from_str_radix(hex, 16) else {
        return Ok(None);
    };
    let high = guid >> 48;
    if high != 0xf130 && high != 0xf140 {
        return Ok(None);
    }
    if let Some(id) = lua
        .app_data_ref::<Metadata>()
        .and_then(|data| data.creatures.get(&guid).copied())
    {
        return Ok(Some(id));
    }
    // A pet's identifier encodes its owner-side pet number, not its creature template.
    if high == 0xf140 {
        return Ok(None);
    }
    let id = ((guid >> 24) & 0xff_ffff) as u32;
    Ok((id != 0).then_some(id))
}

/// 1.12's display divisor per power slot (`0x6e7130`, its table at `0x86f978`): rage 10,
/// happiness 1000, the rest 1.
const POWER_DIVISORS: [u32; 5] = [1, 10, 1, 1, 1000];
/// ClassicAPI's `Enum.PowerType.ComboPoints` and `.Happiness` (`unit/Power.cpp`).
const TYPE_COMBO_POINTS: i64 = 4;
const TYPE_HAPPINESS: i64 = 27;

/// ClassicAPI's power type of a 1.12 power slot: 0 to 3 keep their numbers, happiness (slot 4)
/// is 27 (`Unit::Power::TypeOfSlot`).
pub fn power_type_of_slot(slot: u32) -> i64 {
    if slot == 4 {
        TYPE_HAPPINESS
    } else {
        i64::from(slot)
    }
}

/// The 1.12 slot of a power type, `None` for a type 1.12 keeps no store for.
fn power_slot_of_type(kind: i64) -> Option<usize> {
    match kind {
        0..=3 => Some(kind as usize),
        TYPE_HAPPINESS => Some(4),
        _ => None,
    }
}

/// ClassicAPI's `UnitPower` / `UnitPowerMax` (`unit/Power.cpp`, the 3.80.2 client's): `kind` in
/// `Enum.PowerType`, nil for the unit's display power. The player's combo points (type 4, a rogue
/// or druid's, at most 5) live outside the power block; happiness is type 27. Values are divided
/// by the slot's display divisor unless `raw`. Only the current value is gated: a type other
/// than the unit's display power answers 0 unless the unit is the player or its pet. A unit out
/// of view answers through the group roster's one power, any other type 0.
fn unit_power(
    lua: &Lua,
    token: &str,
    kind: Option<i64>,
    raw: bool,
    maximum: bool,
) -> mlua::Result<u32> {
    let guid = super::auras::unit_guid(lua, token);
    let power = lua
        .app_data_ref::<Metadata>()
        .and_then(|metadata| guid.and_then(|guid| metadata.powers.get(&guid)).cloned());
    let (player, pet) = {
        let model = lua.app_data_ref::<Model>().expect("model");
        (model.unit_guids.player, model.unit_guids.pet)
    };
    if let Some(power) = power {
        let display = power_type_of_slot(u32::from(power.kind));
        let kind = kind.unwrap_or(display);
        let is_player = guid == Some(player) && player != 0;
        if kind == TYPE_COMBO_POINTS {
            if !is_player {
                return Ok(0);
            }
            let (_, class): (Value, Option<String>) = lua
                .globals()
                .get::<mlua::Function>("UnitClass")?
                .call("player")?;
            if !matches!(class.as_deref(), Some("ROGUE" | "DRUID")) {
                return Ok(0);
            }
            if maximum {
                return Ok(5);
            }
            return lua
                .globals()
                .get::<mlua::Function>("GetComboPoints")?
                .call::<Option<u32>>(())
                .map(|points| points.unwrap_or(0));
        }
        let Some(slot) = power_slot_of_type(kind) else {
            return Ok(0);
        };
        let gated = !maximum && kind != display && !is_player && !(pet != 0 && guid == Some(pet));
        if gated {
            return Ok(0);
        }
        let value = if maximum {
            power.maximum[slot]
        } else {
            power.current[slot]
        }
        .unwrap_or(0);
        return Ok(if raw {
            value
        } else {
            value / POWER_DIVISORS[slot]
        });
    }
    // Out of view: the roster's one power, which the stock getters read already divided.
    let active: i64 = lua
        .globals()
        .get::<mlua::Function>("UnitPowerType")?
        .call(token)?;
    let Ok(active_slot) = u32::try_from(active) else {
        return Ok(0);
    };
    if kind.is_some_and(|kind| kind != power_type_of_slot(active_slot)) {
        return Ok(0);
    }
    let stock = if maximum { "UnitManaMax" } else { "UnitMana" };
    let value: u32 = lua.globals().get::<mlua::Function>(stock)?.call(token)?;
    Ok(if raw {
        value.saturating_mul(
            POWER_DIVISORS
                .get(active_slot as usize)
                .copied()
                .unwrap_or(1),
        )
    } else {
        value
    })
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    // ClassicAPI's `UnitInRange(unit)`: within 40 yards of the player, and whether a position
    // check ran; a token with no unit, the player or a unit with no reach answers false, false.
    lua.globals().set(
        "UnitInRange",
        lua.create_function(|lua, unit: String| {
            let Some(guid) = guid(lua, &unit)? else {
                return Ok((false, false));
            };
            let model = lua.app_data_ref::<Model>().expect("model");
            if guid == model.unit_guids.player {
                return Ok((false, false));
            }
            Ok(crate::script::inspect::reach_of(lua, &model, guid)
                .filter(|reach| reach.dist_sq.is_finite())
                .map_or((false, false), |reach| (reach.dist_sq <= 40.0 * 40.0, true)))
        })?,
    )?;
    // ClassicAPI's `C_InstanceEncounter` (`combatlog/BossCreatures.cpp`): the creature IDs an addon
    // names bosses, one ID or a table of them read from [1] to the first nil; a bad entry raises
    // the field error before any is applied. The list lives as long as the VM.
    let encounter = lua.create_table()?;
    fn creature_ids(lua: &Lua, value: Value) -> mlua::Result<Vec<u32>> {
        let one = |v: Value, field: Option<usize>| -> mlua::Result<u32> {
            lua.coerce_integer(v)?
                .and_then(|n| u32::try_from(n).ok())
                .filter(|n| *n > 0)
                .ok_or_else(|| {
                    mlua::Error::runtime(match field {
                        Some(i) => format!("bad argument #1 (Current Field: [{i}])"),
                        None => "bad argument #1 (creatureID expected)".into(),
                    })
                })
        };
        match value {
            Value::Table(list) => {
                let mut ids = Vec::new();
                for i in 1.. {
                    match list.raw_get::<Value>(i)? {
                        Value::Nil => break,
                        v => ids.push(one(v, Some(i))?),
                    }
                }
                Ok(ids)
            }
            v => Ok(vec![one(v, None)?]),
        }
    }
    encounter.set(
        "AddBossCreatureID",
        lua.create_function(|lua, value: Value| {
            let ids = creature_ids(lua, value)?;
            let mut metadata = lua.app_data_mut::<Metadata>().expect("unit metadata");
            for id in ids {
                if !metadata.boss_creatures.contains(&id) {
                    metadata.boss_creatures.push(id);
                }
            }
            Ok(())
        })?,
    )?;
    encounter.set(
        "RemoveBossCreatureID",
        lua.create_function(|lua, value: Value| {
            let ids = creature_ids(lua, value)?;
            lua.app_data_mut::<Metadata>()
                .expect("unit metadata")
                .boss_creatures
                .retain(|id| !ids.contains(id));
            Ok(())
        })?,
    )?;
    encounter.set(
        "GetBossCreatureIDs",
        lua.create_function(|lua, ()| {
            let ids = lua
                .app_data_ref::<Metadata>()
                .expect("unit metadata")
                .boss_creatures
                .clone();
            lua.create_sequence_from(ids)
        })?,
    )?;
    lua.globals().set("C_InstanceEncounter", encounter)?;
    // ClassicAPI's `UnitDistanceSquared(unit)` (`unit/Range.cpp`): the raw squared distance from
    // the player, no reach added, and whether a position was read; the player is (0, true), a
    // unit with no position (0, false), always a number.
    lua.globals().set(
        "UnitDistanceSquared",
        lua.create_function(|lua, unit: Value| {
            let Value::String(unit) = unit else {
                return Err(mlua::Error::runtime("Usage: UnitDistanceSquared(\"unit\")"));
            };
            let Some(guid) = guid(lua, &unit.to_string_lossy())? else {
                return Ok((0.0, false));
            };
            let model = lua.app_data_ref::<Model>().expect("model");
            if guid == model.unit_guids.player {
                return Ok((0.0, true));
            }
            Ok(crate::script::inspect::reach_of(lua, &model, guid)
                .filter(|reach| reach.dist_sq.is_finite())
                .map_or((0.0, false), |reach| (reach.dist_sq, true)))
        })?,
    )?;
    // The inverse of GetSpellName(slot, bookType); the player's book answers before the pet's.
    lua.globals().set(
        "FindSpellBookSlotByID",
        lua.create_function(|lua, id: Value| {
            let Some(id) = lua
                .coerce_number(id)?
                .filter(|n| n.fract() == 0.0 && *n >= 1.0 && *n <= f64::from(u32::MAX))
                .map(|n| n as u32)
            else {
                return Ok((None, None));
            };
            let (spells, pets) = (super::player_book(lua), super::pet_book(lua));
            for (book, slots) in [("spell", &spells.slots), ("pet", &pets.slots)] {
                if let Some(slot) = slots.iter().position(|slot| slot.spell_id == id) {
                    return Ok((Some(slot + 1), Some(book)));
                }
            }
            Ok((None, None))
        })?,
    )?;
    lua.globals().set(
        "GetShapeshiftFormID",
        lua.create_function(|lua, ()| {
            let player = lua
                .app_data_ref::<Model>()
                .expect("model")
                .unit_guids
                .player;
            let metadata = lua.app_data_ref::<Metadata>().expect("unit metadata");
            Ok(metadata.raw.get(&player).map_or(0, |unit| unit.form))
        })?,
    )?;
    // ClassicAPI's `IsSwimming()` (`unit/State.cpp`): the player's `MOVEFLAG_SWIMMING` (0x200000).
    lua.globals().set(
        "IsSwimming",
        lua.create_function(|lua, ()| {
            let metadata = lua.app_data_ref::<Metadata>().expect("unit metadata");
            Ok(metadata.player_move_flags & 0x0020_0000 != 0)
        })?,
    )?;
    // ClassicAPI's `GetSheathState()` (`unit/Sheath.cpp`): the player's sheath state plus one, 1
    // (put away) before the world and for a value past 2.
    lua.globals().set(
        "GetSheathState",
        lua.create_function(|lua, ()| {
            let state = player_raw(lua).map_or(0, |unit| unit.sheath);
            Ok(if state > 2 { 1 } else { state + 1 })
        })?,
    )?;
    lua.globals().set(
        "IsMounted",
        lua.create_function(|lua, ()| {
            Ok(player_raw(lua).is_some_and(|unit| unit.mount_display != 0))
        })?,
    )?;
    // vmangos Player.h: PLAYER_FLAGS_AFK=0x2 and PLAYER_FLAGS_DND=0x4. Read the
    // actual player descriptor; a creature or an unresolved token has neither state.
    for (name, mask) in [("UnitIsAFK", 0x2), ("UnitIsDND", 0x4)] {
        lua.globals().set(
            name,
            lua.create_function(move |lua, token: String| {
                let state = guid(lua, &token)?.and_then(|guid| focus::state(lua, guid));
                Ok(state.is_some_and(|state| state.is_player && state.player_flags & mask != 0))
            })?,
        )?;
    }
    let retained_focus = focus::retained(lua);
    lua.set_app_data(Metadata {
        focus: retained_focus,
        restored_focus_pending: retained_focus.is_some(),
        ..Default::default()
    });
    lua.globals().set(
        "UnitTokenFromGUID",
        lua.create_function(|lua, text: Value| {
            let Value::String(text) = text else {
                return Ok(None);
            };
            let text = text.to_string_lossy();
            let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) else {
                return Ok(None);
            };
            Ok((hex.len() == 16)
                .then(|| u64::from_str_radix(hex, 16).ok())
                .flatten()
                .and_then(|guid| token_from_guid(lua, guid)))
        })?,
    )?;
    for (name, creature_type) in [("UnitCreatureTypeID", true), ("UnitCreatedBySpell", false)] {
        lua.globals().set(
            name,
            lua.create_function(move |lua, token: String| {
                let guid = super::auras::unit_guid(lua, &token);
                let metadata = lua.app_data_ref::<Metadata>().expect("unit metadata");
                Ok(guid
                    .and_then(|guid| metadata.traits.get(&guid))
                    .map(|traits| {
                        if creature_type {
                            traits.creature_type
                        } else {
                            traits.created_by_spell
                        }
                    })
                    .filter(|value| *value != 0))
            })?,
        )?;
    }
    for (name, minion) in [("UnitIsFeignDeath", false), ("UnitIsMinion", true)] {
        lua.globals().set(
            name,
            lua.create_function(move |lua, token: String| {
                let state =
                    super::auras::unit_guid(lua, &token).and_then(|guid| focus::state(lua, guid));
                Ok(state.is_some_and(|state| {
                    if minion {
                        !state.is_player
                            && (state.player_controlled
                                || (state.owner != 0 && state.owner >> 48 == 0))
                    } else {
                        state.flags & 0x2000_0000 != 0
                    }
                }))
            })?,
        )?;
    }
    lua.globals().set(
        "GetPlayerGuid",
        lua.create_function(|lua, ()| {
            Ok(super::auras::unit_guid(lua, "player")
                .filter(|guid| *guid != 0)
                .map(|guid| format!("0x{guid:016X}")))
        })?,
    )?;
    lua.globals().set(
        "GetUnitField",
        lua.create_function(|lua, (token, field): (String, String)| {
            let guid = super::auras::unit_guid(lua, &token);
            let metadata = lua.app_data_ref::<Metadata>().expect("unit metadata");
            let Some(raw) = guid.and_then(|guid| metadata.raw.get(&guid)) else {
                return Ok(Value::Nil);
            };
            match field.as_str() {
                "health" => Ok(raw
                    .health
                    .map_or(Value::Nil, |value| Value::Integer(i64::from(value)))),
                "aura" => Ok(Value::Table(
                    lua.create_sequence_from(raw.auras.iter().copied())?,
                )),
                "auraApplications" => Ok(Value::Table(
                    lua.create_sequence_from(raw.applications.iter().copied())?,
                )),
                _ => Err(mlua::Error::runtime(format!(
                    "unsupported pfUI unit field: {field}"
                ))),
            }
        })?,
    )?;
    lua.globals().set(
        "IsSpellKnown",
        lua.create_function(|lua, (spell, pet): (u32, Option<bool>)| {
            let slots = if pet == Some(true) {
                super::pet_book(lua).slots
            } else {
                super::player_book(lua).slots
            };
            Ok(spell != 0 && slots.iter().any(|slot| slot.spell_id == spell))
        })?,
    )?;
    for (name, raid_only) in [("IsInGroup", false), ("IsInRaid", true)] {
        lua.globals().set(
            name,
            lua.create_function(move |lua, _: mlua::MultiValue| {
                let model = lua.app_data_ref::<Model>().expect("model");
                Ok(!model.party.raid.is_empty() || (!raid_only && !model.party.members.is_empty()))
            })?,
        )?;
    }
    lua.globals().set(
        "UnitSubName",
        lua.create_function(|lua, token: String| {
            let guid = super::auras::unit_guid(lua, &token);
            Ok(guid
                .and_then(|guid| focus::state(lua, guid))
                .and_then(|state| {
                    (!state.is_player)
                        .then_some(state.subtitle)
                        .flatten()
                        .filter(|title| !title.is_empty())
                }))
        })?,
    )?;
    focus::install(lua)?;
    for (name, maximum) in [("UnitPower", false), ("UnitPowerMax", true)] {
        lua.globals().set(
            name,
            lua.create_function(
                move |lua, (token, kind, raw): (String, Option<i64>, Option<bool>)| {
                    unit_power(lua, &token, kind, raw == Some(true), maximum)
                },
            )?,
        )?;
    }
    lua.globals().set(
        "IsPlayerGuid",
        lua.create_function(|lua, token: Option<String>| {
            let guid = token
                .as_deref()
                .and_then(|token| super::auras::unit_guid(lua, token));
            let player = lua
                .app_data_ref::<Model>()
                .expect("model")
                .unit_guids
                .player;
            Ok(player != 0 && guid == Some(player))
        })?,
    )
}
