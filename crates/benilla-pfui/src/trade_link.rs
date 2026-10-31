//! Profession links (`|Htrade:…|h`), the engine half of the ClassicAPI addon's TradeSkillLink.lua:
//! `GetTradeSkillListLink` and `GetCraftListLink` build the player's link for the open window as
//! ClassicAPI builds it, from 3.3.5's encoder, so a ClassicAPI client opens it:
//!
//! `|cffffd000|Htrade:<spellID>:<rank>:<maxRank>:<GUID>:<bits>|h[<spell name>]|h|r`
//!
//! `spellID` is the profession spell that opens the window (its first effect the trade skill),
//! `rank` and `maxRank` the window's, `GUID` the player's in upper-case hex, and `bits` one bit per
//! `SkillLineAbility.dbc` row of the skill line in row order, six to a base64 character, low bit
//! first: set when the player knows the row's spell and the row's race and class masks take the
//! player (3.3.5's encoder ignores the rows' exclusion flags).

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;

use mlua::Lua;

use crate::creature::{number, rows, string};

/// `SPELL_EFFECT_TRADE_SKILL`, the first effect of a profession window's opener.
const TRADE_SKILL: f64 = 47.0;

/// 3.3.5's `s_base64Encode`.
const BASE64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// One `SkillLineAbility.dbc` row of a line: its spell and its race and class masks.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Row {
    spell: u32,
    race_mask: u32,
    class_mask: u32,
}

impl Row {
    /// 3.3.5's `SpellTableMatchRaceClass` with no exclusions.
    fn admits(&self, race: u32, class: u32) -> bool {
        let takes =
            |mask: u32, id: u32| mask == 0 || (1..=32).contains(&id) && mask & (1 << (id - 1)) != 0;
        takes(self.race_mask, race) && takes(self.class_mask, class)
    }
}

/// An opener: its craft type (`EffectMiscValue[0]`, 0 for the TradeSkill window) and name.
#[derive(Clone, Debug, PartialEq)]
struct Opener {
    craft_type: u32,
    name: String,
}

/// The rows and ids the links are built from.
#[derive(Clone, Debug, Default)]
pub struct Catalog {
    /// Each line's rows in row-id order.
    lines: BTreeMap<u32, Vec<Row>>,
    /// Each spell's rows, as `(line, row)`.
    by_spell: HashMap<u32, Vec<(u32, Row)>>,
    /// Race and class file tokens, lower-cased, to their ids.
    races: HashMap<String, u32>,
    classes: HashMap<String, u32>,
    openers: BTreeMap<u32, Opener>,
}

impl Catalog {
    pub fn read(
        read: &dyn Fn(&str) -> Option<Vec<u8>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut catalog = Self::default();
        let abilities = rows(read, "SkillLineAbility", 15)?;
        let mut ordered = Vec::new();
        for record in abilities.records() {
            ordered.push((
                number(record, 0)?,
                number(record, 1)?,
                Row {
                    spell: number(record, 2)?,
                    race_mask: number(record, 3)?,
                    class_mask: number(record, 4)?,
                },
            ));
        }
        ordered.sort_by_key(|(id, _, _)| *id);
        for (_, line, row) in ordered {
            catalog.lines.entry(line).or_default().push(row);
            catalog
                .by_spell
                .entry(row.spell)
                .or_default()
                .push((line, row));
        }
        let races = rows(read, "ChrRaces", 29)?;
        for record in races.records() {
            catalog.races.insert(
                string(&races, record, 15)?.to_lowercase(),
                number(record, 0)?,
            );
        }
        let classes = rows(read, "ChrClasses", 17)?;
        for record in classes.records() {
            catalog.classes.insert(
                string(&classes, record, 14)?.to_lowercase(),
                number(record, 0)?,
            );
        }
        Ok(catalog)
    }

    /// Install the link functions, the openers taken from the spell catalog.
    pub fn install(&self, lua: &Lua, spells: &crate::spell::Catalog) -> mlua::Result<()> {
        let mut catalog = self.clone();
        catalog.openers = spells
            .0
            .iter()
            .filter(|(_, spell)| {
                spell
                    .effects
                    .first()
                    .and_then(|effect| effect.get("effect"))
                    == Some(&TRADE_SKILL)
            })
            .map(|(id, spell)| {
                let craft_type = spell.effects[0]
                    .get("miscValue")
                    .map_or(0, |misc| misc.max(0.0) as u32);
                (
                    *id,
                    Opener {
                        craft_type,
                        name: spell.name.clone(),
                    },
                )
            })
            .collect();
        let catalog = Rc::new(catalog);
        let globals = lua.globals();
        let links = catalog.clone();
        globals.set(
            "GetTradeSkillListLink",
            lua.create_function(move |lua, ()| Ok(links.link(lua, false)))?,
        )?;
        let links = catalog;
        globals.set(
            "GetCraftListLink",
            lua.create_function(move |lua, ()| Ok(links.link(lua, true)))?,
        )?;
        // This client shows only the player's own professions, so no window is linked.
        for name in ["IsTradeSkillLinked", "IsCraftLinked"] {
            globals.set(name, lua.create_function(|_, ()| Ok(false))?)?;
        }
        Ok(())
    }

    /// The player's race and class ids.
    fn player(&self, lua: &Lua) -> Option<(u32, u32)> {
        let token = |function: &str| -> Option<String> {
            let (_, token): (Option<String>, Option<String>) = lua
                .globals()
                .get::<mlua::Function>(function)
                .ok()?
                .call("player")
                .ok()?;
            token.map(|token| token.to_lowercase())
        };
        Some((
            *self.races.get(&token("UnitRace")?)?,
            *self.classes.get(&token("UnitClass")?)?,
        ))
    }

    /// The link for the open TradeSkill window, or the open Craft window with `craft`.
    fn link(&self, lua: &Lua, craft: bool) -> Option<String> {
        use benilla_ui::script::pfui_host::{professions, units};
        let (open, rank, max_rank) = if craft {
            professions::open_craft(lua)?
        } else {
            professions::open_trade_skill(lua)?
        };
        let (race, class) = self.player(lua)?;
        let known: BTreeSet<u32> = units::known_spells(lua).into_iter().collect();
        // The opener: a known profession spell of the window's kind, its line from its row for the
        // player; the TradeSkill window's must be the open line.
        let (spell, line) = known.iter().rev().find_map(|spell| {
            let opener = self.openers.get(spell)?;
            if opener.craft_type != if craft { open } else { 0 } {
                return None;
            }
            let (line, _) = self
                .by_spell
                .get(spell)?
                .iter()
                .find(|(_, row)| row.admits(race, class))?;
            (craft || *line == open).then_some((*spell, *line))
        })?;
        let rows = self.lines.get(&line)?;
        let guid = units::player_guid(lua);
        if guid == 0 {
            return None;
        }
        Some(format!(
            "|cffffd000|Htrade:{spell}:{rank}:{max_rank}:{guid:X}:{}|h[{}]|h|r",
            bits(rows, &known, race, class),
            self.openers[&spell].name
        ))
    }
}

/// The rows' bits, six to a base64 character, low bit first.
fn bits(rows: &[Row], known: &BTreeSet<u32>, race: u32, class: u32) -> String {
    rows.chunks(6)
        .map(|chunk| {
            let value = chunk.iter().enumerate().fold(0usize, |value, (bit, row)| {
                if known.contains(&row.spell) && row.admits(race, class) {
                    value | 1 << bit
                } else {
                    value
                }
            });
            char::from(BASE64[value])
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(spell: u32, race_mask: u32, class_mask: u32) -> Row {
        Row {
            spell,
            race_mask,
            class_mask,
        }
    }

    #[test]
    fn the_bits_are_six_rows_to_a_character_low_bit_first() {
        let rows: Vec<Row> = (1..=8).map(|spell| row(spell, 0, 0)).collect();
        let known = BTreeSet::from([1, 3, 7, 8]);
        // 0b000101 = 5 'F', then rows 7 and 8: 0b11 = 3 'D'.
        assert_eq!(bits(&rows, &known, 1, 1), "FD");
        assert_eq!(bits(&rows, &BTreeSet::new(), 1, 1), "AA");
    }

    fn opener(name: &str, craft_type: f64) -> crate::spell::Spell {
        let mut effects = vec![BTreeMap::new(); 3];
        effects[0].insert("effect", TRADE_SKILL);
        effects[0].insert("miscValue", craft_type);
        crate::spell::Spell {
            name: name.into(),
            rank: String::new(),
            icon: None,
            dispel: 0,
            attributes: 0,
            reagents: Vec::new(),
            effects,
            cast_time: 0,
            min_range: 0.0,
            max_range: 0.0,
            cost: 0,
            power_type: 0,
            funnel: false,
            swing: None,
        }
    }

    #[test]
    fn the_open_windows_link_names_its_opener_ranks_guid_and_known_rows() {
        use benilla_ui::script::{pfui_host::units, TradeSkillState, UiScript, UnitGuids};
        let mut script = UiScript::new().unwrap();
        crate::install(&script).unwrap();
        let spells = crate::spell::Catalog(BTreeMap::from([
            (3908, opener("Tailoring", 0.0)),
            (7411, opener("Enchanting", 3.0)),
        ]));
        spells.install(script.lua()).unwrap();
        let tailoring = [row(3908, 0, 0), row(2387, 0, 0), row(2393, 0, 0)];
        Catalog {
            lines: BTreeMap::from([(197, tailoring.to_vec())]),
            by_spell: tailoring
                .iter()
                .map(|row| (row.spell, vec![(197, *row)]))
                .collect(),
            races: HashMap::from([("human".into(), 1)]),
            classes: HashMap::from([("mage".into(), 8)]),
            openers: BTreeMap::new(),
        }
        .install(script.lua(), &spells)
        .unwrap();
        script.set_unit_guids(&UnitGuids {
            player: 0x1234,
            ..Default::default()
        });
        units::set_known_spells(script.lua(), [3908, 2387]);
        script
            .run(
                "UnitRace = function() return 'Human', 'Human' end
                 UnitClass = function() return 'Mage', 'MAGE' end
                 assert(GetTradeSkillListLink() == nil)",
            )
            .unwrap();
        script.set_trade_skill(Some(TradeSkillState {
            line: 197,
            line_name: "Tailoring".into(),
            rank: 75,
            max_rank: 150,
            recipes: Vec::new(),
        }));
        script
            .run(
                r#"
            local link = GetTradeSkillListLink()
            assert(link == "|cffffd000|Htrade:3908:75:150:1234:D|h[Tailoring]|h|r", link)
            assert(GetCraftListLink() == nil and IsTradeSkillLinked() == false)
        "#,
            )
            .unwrap();
    }

    #[test]
    fn a_row_another_race_or_class_holds_is_not_set() {
        let rows = [row(1, 1 << 1, 0), row(2, 0, 1 << 7), row(3, 1, 1 << 7)];
        let known = BTreeSet::from([1, 2, 3]);
        // A human (1) mage (8): row 1 is orcs' only.
        assert_eq!(bits(&rows, &known, 1, 8), "G");
    }
}
