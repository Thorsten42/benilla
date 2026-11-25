//! ClassicAPI's server-side aura duration modifiers (`aura/Source.cpp`, `turtle/DurationMods.cpp`):
//! a server mechanic that changes the remaining time of a caster's aura on another unit sends the
//! observing caster nothing, but the triggering cast's spell-go is seen, so a rule mirrors the edit.
//!
//! A rule's trigger is an exact spell ID, or a SpellFamilyName plus a school index (below 0 for any
//! school); the aura it edits is the caster's aura on each unit the cast hit whose spell has the
//! rule's SpellFamilyName and overlaps its SpellFamilyFlags mask and/or has its SpellIconID (a zero
//! mask or icon drops that half; both zero is refused). The edit is `refresh` (back to the full
//! duration), `reduce` (by the value, unless that reaches the end), `set` (to the value) or
//! `remove` (left to the aura's removal, which the descriptor reports).
//!
//! Addons register family/school rules with
//! `C_UnitAuras.RegisterAuraDurationModifierByTrigger(triggerFamily, triggerSchool,
//! affectedFamily, affectedFamilyFlags, affectedIcon, op [, valueSeconds])`, which answers whether
//! the rule was taken (at most 128). ClassicAPI's own addon registers Shadow Weaving's this way.
//! The host adds Turtle's built-in exact-ID rules ([`TURTLE`]) and applies every rule at a spell-go.
// Deviation: ClassicAPI also arms a trigger's targets so a rule re-applies to an aura the trigger
// itself creates moments later (Bestial Wrath's Scent of Blood); here a rule edits only an aura
// that is already there.

use mlua::{Lua, Value};

const MAX_RULES: usize = 128;

/// What a rule does to the aura it matches.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Op {
    Refresh,
    /// Shave this many seconds off the remaining time.
    Reduce(f64),
    /// Set the duration and the remaining time to this many seconds.
    Set(f64),
    Remove,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rule {
    /// The exact trigger; 0 matches by [`Self::trigger_family`] and [`Self::trigger_school`].
    pub trigger_spell: u32,
    pub trigger_family: u32,
    /// The trigger's school index; below 0 for any school.
    pub trigger_school: i32,
    pub family: u32,
    pub mask: u64,
    pub icon: u32,
    pub op: Op,
}

/// The Spell.dbc facts a rule reads: SpellFamilyName, SpellFamilyFlags, SpellIconID, School.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Facts {
    pub family: u32,
    pub flags: u64,
    pub icon: u32,
    pub school: u32,
}

impl Rule {
    pub fn triggered_by(&self, spell: u32, facts: Option<Facts>) -> bool {
        if self.trigger_spell != 0 {
            return self.trigger_spell == spell;
        }
        facts.is_some_and(|f| {
            f.family == self.trigger_family
                && (self.trigger_school < 0 || f.school as i32 == self.trigger_school)
        })
    }

    pub fn affects(&self, facts: Option<Facts>) -> bool {
        facts.is_some_and(|f| {
            f.family == self.family
                && (self.mask == 0 || f.flags & self.mask != 0)
                && (self.icon == 0 || f.icon == self.icon)
        })
    }
}

/// `SpellFamilyName`s of the built-in rules.
const WARLOCK: u32 = 5;
const HUNTER: u32 = 9;
const SHAMAN: u32 = 11;

/// Turtle WoW's built-in rules: Conflagrate shaves 3 s off Immolate (Turtle keeps it ticking),
/// Molten Blast refreshes Flame Shock, and Bestial Wrath sets Scent of Blood to its own duration
/// (`bestial_wrath` seconds, from Bestial Wrath's record).
pub fn turtle(bestial_wrath: f64) -> Vec<Rule> {
    let exact = |trigger_spell, family, mask, icon, op| Rule {
        trigger_spell,
        trigger_family: 0,
        trigger_school: -1,
        family,
        mask,
        icon,
        op,
    };
    let mut rules = Vec::new();
    for id in [17962, 18930, 18931, 18932] {
        rules.push(exact(id, WARLOCK, 0x4, 0, Op::Reduce(3.0)));
    }
    for id in 36916..=36921 {
        rules.push(exact(id, SHAMAN, 0x1000_0000, 0, Op::Refresh));
    }
    rules.push(exact(19574, HUNTER, 0, 2245, Op::Set(bestial_wrath)));
    rules
}

/// [`turtle`]'s Bestial Wrath, for the host to read its duration.
pub const BESTIAL_WRATH: u32 = 19574;

#[derive(Default)]
struct Rules(Vec<Rule>);

/// The rules addons registered, in order.
pub fn registered(lua: &Lua) -> Vec<Rule> {
    lua.app_data_ref::<Rules>()
        .map(|r| r.0.clone())
        .unwrap_or_default()
}

fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Integer(n) => Some(*n as f64),
        Value::Number(n) => Some(*n),
        // Lua coerces a numeric string where a number is asked.
        Value::String(s) => s.to_str().ok()?.trim().parse().ok(),
        _ => None,
    }
}

pub(crate) fn install(lua: &Lua) -> mlua::Result<()> {
    lua.set_app_data(Rules::default());
    let api: mlua::Table = lua.globals().get("C_UnitAuras")?;
    api.set(
        "RegisterAuraDurationModifierByTrigger",
        lua.create_function(|lua, args: mlua::Variadic<Value>| {
            let at = |i: usize| args.get(i).unwrap_or(&Value::Nil);
            let nums: Option<Vec<f64>> = (0..5).map(|i| number(at(i))).collect();
            let (Some(nums), Value::String(op)) = (nums, at(5)) else {
                return Err(mlua::Error::runtime(
                    "Usage: C_UnitAuras.RegisterAuraDurationModifierByTrigger(triggerFamily, \
                     triggerSchool, affectedFamily, affectedFamilyFlags, affectedIcon, op[, \
                     valueSeconds])",
                ));
            };
            let value = number(at(6)).unwrap_or(0.0);
            let op = match op.to_str()?.to_ascii_lowercase().as_str() {
                "refresh" => Op::Refresh,
                "reduce" => Op::Reduce(value),
                "set" => Op::Set(value),
                "remove" => Op::Remove,
                _ => return Ok(false),
            };
            let rule = Rule {
                trigger_spell: 0,
                trigger_family: nums[0] as u32,
                trigger_school: nums[1] as i32,
                family: nums[2] as u32,
                mask: nums[3] as u64,
                icon: nums[4] as u32,
                op,
            };
            if rule.trigger_family == 0 || (rule.mask == 0 && rule.icon == 0) {
                return Ok(false);
            }
            let Some(mut rules) = lua.app_data_mut::<Rules>() else {
                return Ok(false);
            };
            if rules.0.contains(&rule) {
                return Ok(true);
            }
            if rules.0.len() >= MAX_RULES {
                return Ok(false);
            }
            rules.0.push(rule);
            Ok(true)
        })?,
    )
}
