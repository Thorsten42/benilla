//! ClassicAPI's death recap (`combatlog/DeathRecap.cpp`), built from the kept combat log as
//! 3.80.2's client builds it: at the player's death, walk the log newest first over the entries
//! whose target is the player, back to the previous death; keep at most 5 damage entries whose
//! amount past the overkill is above 0, and every `SPELL_INSTAKILL`; an entry with overkill
//! starts the list over, so the killing hit comes first. Each death takes the next recap ID, and
//! one with entries is kept, with the player's maximum health, in the next of 15 slots. Every
//! `UNIT_DIED` / `UNIT_DESTROYED` carries `recapID, unconsciousOnDeath`: the ID for the player's
//! death, -1 for any other, and 0 (1.12 has no unconscious death).
//!
//! `C_DeathRecap.GetRecapEvents`, `GetRecapLink`, `GetRecapMaxHealth` and `HasRecapEvents` find a
//! recap by ID, the latest with none given. Logging out empties the slots and restarts the IDs.
// Deviation: a drain's amount is read as ClassicAPI reads it (its power type), and its overkill is
// 0, as this log carries no alternate power type.

use benilla_ui::script::ScriptValue;
use mlua::{Lua, Table, Value};

use super::{Entry, Log};

const RECAPS: usize = 15;
const EVENTS: usize = 5;

/// The damage events a recap keeps (ClassicAPI's `kDamageEvents` mask, by name).
const DAMAGE: [&str; 11] = [
    "ENVIRONMENTAL_DAMAGE",
    "SWING_DAMAGE",
    "RANGE_DAMAGE",
    "SPELL_DAMAGE",
    "SPELL_DRAIN",
    "SPELL_LEECH",
    "SPELL_PERIODIC_DAMAGE",
    "SPELL_PERIODIC_DRAIN",
    "SPELL_PERIODIC_LEECH",
    "DAMAGE_SHIELD",
    "DAMAGE_SPLIT",
];

struct Recap {
    id: i64,
    max_health: f64,
    /// Each kept entry's values and its `currentHP`, newest first.
    events: Vec<(Vec<ScriptValue>, &'static str, u32)>,
}

#[derive(Default)]
pub(super) struct Recaps {
    slots: Vec<Option<Recap>>,
    last_id: i64,
    slot: usize,
}

/// How many values the subevent's prefix has before its suffix.
fn prefix_len(subevent: &str) -> usize {
    if subevent.starts_with("SWING_") {
        0
    } else if subevent.starts_with("ENVIRONMENTAL_") {
        1
    } else {
        3
    }
}

fn is_drain(subevent: &str) -> bool {
    subevent.ends_with("_DRAIN") || subevent.ends_with("_LEECH")
}

fn num(v: Option<&ScriptValue>) -> i64 {
    match v {
        Some(ScriptValue::Int(n)) => *n,
        Some(ScriptValue::Number(n)) => *n as i64,
        _ => 0,
    }
}

/// ClassicAPI's `Amount` and `Overkill` of an entry, from the values after the eleven common ones.
fn amount_and_overkill(subevent: &str, values: &[ScriptValue]) -> (i64, i64) {
    let p = prefix_len(subevent);
    if is_drain(subevent) {
        (num(values.get(p + 1)), 0)
    } else {
        (num(values.get(p)), num(values.get(p + 1)))
    }
}

fn player_guid(lua: &Lua) -> Option<u64> {
    let text: Option<String> = lua
        .globals()
        .get::<mlua::Function>("UnitGUID")
        .and_then(|f| f.call(("player",)))
        .ok()?;
    let hex = text?;
    u64::from_str_radix(hex.trim_start_matches("0x").trim_start_matches("0X"), 16).ok()
}

/// The recap ID a death of `dest` carries: the player's makes one from the kept log, before its
/// own entry is kept, so the walk stops at the previous death.
pub(super) fn on_death(lua: &Lua, dest: Option<u64>) -> i64 {
    let Some(player) = player_guid(lua).filter(|p| dest == Some(*p)) else {
        return -1;
    };
    let max_health = lua
        .globals()
        .get::<mlua::Function>("UnitHealthMax")
        .and_then(|f| f.call::<f64>(("player",)))
        .unwrap_or(0.0);
    let Some(mut log) = lua.app_data_mut::<Log>() else {
        return -1;
    };
    let mut kept: Vec<(Vec<ScriptValue>, &'static str, u32)> = Vec::new();
    for Entry { event, args, .. } in log.entries.iter().rev() {
        if event.dest.as_ref().map(|d| d.guid) != Some(player) {
            continue;
        }
        if matches!(
            event.subevent,
            "UNIT_DIED" | "UNIT_DESTROYED" | "UNIT_DISSIPATES"
        ) {
            break;
        }
        let mut keep = event.subevent == "SPELL_INSTAKILL";
        if DAMAGE.contains(&event.subevent) {
            let (amount, overkill) = amount_and_overkill(event.subevent, &event.values);
            let net = if overkill < 1 {
                amount
            } else {
                amount - overkill
            };
            if net >= 1 {
                if overkill > 0 {
                    kept.clear();
                }
                keep = true;
            }
        }
        if keep {
            kept.push((args.clone(), event.subevent, event.dest_health.unwrap_or(0)));
        }
        if kept.len() >= EVENTS {
            break;
        }
    }
    let recaps = &mut log.recaps;
    recaps.last_id += 1;
    let id = recaps.last_id;
    if kept.is_empty() {
        return -1;
    }
    if recaps.slots.len() < RECAPS {
        recaps.slots.resize_with(RECAPS, || None);
    }
    recaps.slot = (recaps.slot + 1) % RECAPS;
    let slot = recaps.slot;
    recaps.slots[slot] = Some(Recap {
        id,
        max_health,
        events: kept,
    });
    id
}

fn find(log: &Log, id: Option<i64>) -> Option<&Recap> {
    let id = id.unwrap_or(log.recaps.last_id);
    if id <= 0 {
        return None;
    }
    log.recaps.slots.iter().flatten().find(|r| r.id == id)
}

fn recap_id(v: &Value) -> Option<i64> {
    match v {
        Value::Integer(n) => Some(*n),
        Value::Number(n) => Some(*n as i64),
        _ => None,
    }
}

/// One kept entry as a recap event table: the common fields, the prefix's and the suffix's.
fn event_table(
    lua: &Lua,
    (args, subevent, current_hp): &(Vec<ScriptValue>, &'static str, u32),
) -> mlua::Result<Table> {
    let t = lua.create_table()?;
    let at = |i: usize| super::lua_value(lua, args.get(i).unwrap_or(&ScriptValue::Nil));
    for (i, key) in [
        "timestamp",
        "event",
        "hideCaster",
        "sourceGUID",
        "sourceName",
        "sourceFlags",
        "sourceRaidFlags",
        "destGUID",
        "destName",
        "destFlags",
        "destRaidFlags",
    ]
    .into_iter()
    .enumerate()
    {
        t.set(key, at(i)?)?;
    }
    let p = prefix_len(subevent);
    let base = 11;
    if p == 3 {
        t.set("spellId", at(base)?)?;
        t.set("spellName", at(base + 1)?)?;
        t.set("spellSchool", at(base + 2)?)?;
    } else if p == 1 {
        t.set("environmentalType", at(base)?)?;
    }
    let s = base + p;
    if is_drain(subevent) {
        t.set("amount", at(s)?)?;
        t.set("powerType", at(s + 1)?)?;
        t.set("extraAmount", at(s + 2)?)?;
    } else if *subevent != "SPELL_INSTAKILL" {
        for (i, key) in [
            "amount", "overkill", "school", "resisted", "blocked", "absorbed",
        ]
        .into_iter()
        .enumerate()
        {
            t.set(key, at(s + i)?)?;
        }
        for (i, key) in ["critical", "glancing", "crushing", "isOffHand"]
            .into_iter()
            .enumerate()
        {
            let on = !matches!(args.get(s + 6 + i), None | Some(ScriptValue::Nil));
            t.set(key, on)?;
        }
    }
    t.set("currentHP", *current_hp)?;
    Ok(t)
}

pub(super) fn install(lua: &Lua) -> mlua::Result<()> {
    let api = lua.create_table()?;
    api.set(
        "GetRecapEvents",
        lua.create_function(|lua, id: Value| {
            let log = lua.app_data_ref::<Log>();
            let Some(recap) = log.as_deref().and_then(|l| find(l, recap_id(&id))) else {
                return Ok(mlua::MultiValue::new());
            };
            let list = lua.create_table()?;
            for (i, event) in recap.events.iter().enumerate() {
                list.raw_set(i + 1, event_table(lua, event)?)?;
            }
            Ok(mlua::MultiValue::from_vec(vec![Value::Table(list)]))
        })?,
    )?;
    api.set(
        "GetRecapLink",
        lua.create_function(|lua, id: Value| {
            let found = lua
                .app_data_ref::<Log>()
                .and_then(|l| find(&l, recap_id(&id)).map(|r| r.id));
            let global = |name: &str| -> String {
                lua.globals()
                    .get::<Option<String>>(name)
                    .ok()
                    .flatten()
                    .unwrap_or_default()
            };
            match found {
                Some(id) => {
                    let format: mlua::Function =
                        lua.globals().get::<Table>("string")?.get("format")?;
                    format.call::<String>((global("DEATH_RECAP_LINK"), id))
                }
                None => Ok(global("DEATH_RECAP_TEXT")),
            }
        })?,
    )?;
    api.set(
        "GetRecapMaxHealth",
        lua.create_function(|lua, id: Value| {
            Ok(lua
                .app_data_ref::<Log>()
                .and_then(|l| find(&l, recap_id(&id)).map(|r| r.max_health))
                .unwrap_or(0.0))
        })?,
    )?;
    api.set(
        "HasRecapEvents",
        lua.create_function(|lua, id: Value| {
            Ok(lua
                .app_data_ref::<Log>()
                .is_some_and(|l| find(&l, recap_id(&id)).is_some()))
        })?,
    )?;
    lua.globals().set("C_DeathRecap", api)
}
