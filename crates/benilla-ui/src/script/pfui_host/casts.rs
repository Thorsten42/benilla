//! Observed cast/channel snapshots and the modern lifecycle event projection.

use crate::script::{Model, ScriptValue};
use mlua::{Lua, MultiValue, Table, Value};
use std::collections::HashMap;

#[derive(Clone)]
pub struct Cast {
    pub spell_id: u32,
    pub name: String,
    pub rank: String,
    pub icon: Option<String>,
    pub start_ms: f64,
    pub end_ms: f64,
    pub tradeskill: bool,
    pub cast_guid: String,
    pub delay_ms: u32,
    pub channel: bool,
    /// An interrupt or a silence can stop it (`PreventionType` silence).
    pub silenceable: bool,
}

#[derive(Default)]
struct Extras(HashMap<u64, Cast>);

/// Each caster's current cast's unit target, as its `SMSG_SPELL_START` named it.
#[derive(Default)]
struct Targets(HashMap<u64, u64>);

/// Replace the casts' unit targets, beside [`set_casts`].
pub fn set_cast_targets(lua: &Lua, targets: impl IntoIterator<Item = (u64, u64)>) {
    lua.set_app_data(Targets(targets.into_iter().collect()));
}

pub fn player_cast(lua: &Lua) -> Option<Cast> {
    let me = lua.app_data_ref::<Model>()?.unit_guids.player;
    lua.app_data_ref::<Extras>()?.0.get(&me).cloned()
}

pub fn set_casts(lua: &Lua, casts: impl IntoIterator<Item = (u64, Cast)>) {
    // The player's cast shows the castGUID its `UNIT_SPELLCAST_SENT` minted, once its first event
    // took it.
    let casts: Vec<(u64, Cast)> = casts
        .into_iter()
        .map(|(caster, mut cast)| {
            cast.cast_guid = shown_guid(lua, caster, &cast, false);
            (caster, cast)
        })
        .collect();
    if let Some(mut extras) = lua.app_data_mut::<Extras>() {
        extras.0.clear();
        extras.0.extend(casts);
    }
}

fn info(lua: &Lua, token: &str, channel: bool) -> mlua::Result<MultiValue> {
    let guid = super::auras::unit_guid(lua, token);
    let Some(extras) = lua.app_data_ref::<Extras>() else {
        return Ok(MultiValue::new());
    };
    let Some((guid, cast)) = guid
        .and_then(|guid| extras.0.get(&guid).map(|cast| (guid, cast)))
        .filter(|(_, cast)| cast.channel == channel)
    else {
        return Ok(MultiValue::new());
    };
    let mut values = vec![
        Value::String(lua.create_string(&cast.name)?),
        Value::String(lua.create_string(&cast.name)?),
        cast.icon
            .as_ref()
            .map(|icon| lua.create_string(icon))
            .transpose()?
            .map_or(Value::Nil, Value::String),
        Value::Number(cast.start_ms),
        Value::Number(cast.end_ms),
        Value::Boolean(cast.tradeskill),
    ];
    if !channel {
        values.push(Value::String(lua.create_string(&cast.cast_guid)?));
    }
    values.push(Value::Boolean(super::interrupts::not_interruptible(
        lua, guid, cast,
    )));
    values.push(Value::Integer(i64::from(cast.spell_id)));
    if !channel {
        values.extend([Value::Nil, Value::Integer(i64::from(cast.delay_ms))]);
    }
    Ok(MultiValue::from_vec(values))
}

pub fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Extras::default());
    // `UnitSpellTargetName(unit)`: the name of the unit the unit's cast is aimed at; nil without
    // a cast, without a unit target (self, ground, item) or with nobody to name.
    lua.globals().set(
        "UnitSpellTargetName",
        lua.create_function(|lua, unit: String| {
            let Some(caster) = super::units::guid(lua, &unit)? else {
                return Ok(None);
            };
            let casting = lua
                .app_data_ref::<Extras>()
                .is_some_and(|extras| extras.0.contains_key(&caster));
            let target = lua
                .app_data_ref::<Targets>()
                .and_then(|targets| targets.0.get(&caster).copied());
            Ok(target
                .filter(|_| casting)
                .and_then(|target| super::units::name_of(lua, target)))
        })?,
    )?;
    let api: Table = lua.globals().get("C_Spell")?;
    api.set(
        "UnitCastingInfo",
        lua.create_function(|lua, unit: String| info(lua, &unit, false))?,
    )?;
    api.set(
        "UnitChannelInfo",
        lua.create_function(|lua, unit: String| info(lua, &unit, true))?,
    )?;
    api.set(
        "ChannelInfo",
        lua.create_function(|lua, ()| info(lua, "player", true))?,
    )?;
    Ok(())
}

/// The castGUIDs `UNIT_SPELLCAST_SENT` minted: the last send's spell and GUID, which the player's
/// next cast of that spell takes, and each taken cast's own GUID with the minted one it shows.
#[derive(Default)]
struct SentCasts {
    serial: u64,
    pending: Option<(u32, String)>,
    /// `(cast's own GUID, minted GUID)`, newest last; a few casts back is plenty.
    aliases: std::collections::VecDeque<(String, String)>,
}

/// How many taken casts keep their minted GUID.
const SENT_ALIASES: usize = 16;

/// ClassicAPI's `UNIT_SPELLCAST_SENT` (`spell/CastEvents.cpp`, `OnSend`): the arguments for the
/// player's spell send, `unit, target, castGUID, spellID, spellName, rank`, with `target` the first
/// token naming the cast's unit target ("" for none). The castGUID is minted here and pending: the
/// player's next START, CHANNEL_START, SUCCEEDED or FAILED of that spell takes it, and every later
/// event of that cast, and the cast getters, show it (`g_pendingGuid`).
pub fn sent(
    lua: &Lua,
    spell_id: u32,
    target: Option<u64>,
    name: String,
    rank: String,
) -> Vec<ScriptValue> {
    let target = target
        .and_then(|guid| super::units::event_tokens(lua, guid).into_iter().next())
        .unwrap_or_default();
    let player = super::auras::unit_guid(lua, "player").unwrap_or(0);
    let cast_guid = {
        if lua.app_data_ref::<SentCasts>().is_none() {
            lua.set_app_data(SentCasts::default());
        }
        let mut sent = lua.app_data_mut::<SentCasts>().expect("set above");
        sent.serial += 1;
        let cast_guid = format!("Cast-{player:016X}-S{}", sent.serial);
        sent.pending = Some((spell_id, cast_guid.clone()));
        cast_guid
    };
    vec![
        ScriptValue::Str("player".into()),
        ScriptValue::Str(target),
        ScriptValue::Str(cast_guid),
        ScriptValue::Int(i64::from(spell_id)),
        ScriptValue::Str(name),
        ScriptValue::Str(rank),
    ]
}

/// The castGUID the player's cast shows: its minted one once taken, taking the pending one on a
/// cast's first event (`claim`).
fn shown_guid(lua: &Lua, caster: u64, cast: &Cast, claim: bool) -> String {
    if super::auras::unit_guid(lua, "player") != Some(caster) {
        return cast.cast_guid.clone();
    }
    let Some(mut sent) = lua.app_data_mut::<SentCasts>() else {
        return cast.cast_guid.clone();
    };
    if let Some((_, minted)) = sent.aliases.iter().find(|(own, _)| *own == cast.cast_guid) {
        return minted.clone();
    }
    if !claim
        || !sent
            .pending
            .as_ref()
            .is_some_and(|(spell, _)| *spell == cast.spell_id)
    {
        return cast.cast_guid.clone();
    }
    let (_, minted) = sent.pending.take().expect("checked");
    if sent.aliases.len() == SENT_ALIASES {
        sent.aliases.pop_front();
    }
    sent.aliases
        .push_back((cast.cast_guid.clone(), minted.clone()));
    minted
}

/// Fire once for each current stock token naming the caster, preserving legacy argN values.
pub fn event(lua: &Lua, caster: u64, kind: &str, cast: &Cast) {
    let claim = matches!(
        kind,
        "UNIT_SPELLCAST_START"
            | "UNIT_SPELLCAST_CHANNEL_START"
            | "UNIT_SPELLCAST_SUCCEEDED"
            | "UNIT_SPELLCAST_FAILED"
    );
    let cast_guid = shown_guid(lua, caster, cast, claim);
    for token in super::units::event_tokens(lua, caster) {
        crate::script::pfui_host::fire_pfui_event(
            lua,
            kind,
            vec![
                ScriptValue::Str(token),
                ScriptValue::Str(cast_guid.clone()),
                ScriptValue::Int(i64::from(cast.spell_id)),
                ScriptValue::Str(cast.name.clone()),
                ScriptValue::Str(cast.rank.clone()),
            ],
        );
    }
}
