//! ClassicAPI's spell descriptions (`spell/Description.cpp`): `C_Spell.GetSpellDescription(spellID)`
//! is the tooltip's description text for any spell, its `$`-tokens at the spell's base values (no
//! caster level, spell power or talents), nil for a spell with no description. The host expands the
//! tokens ([`SpellDescriptionQuery`]); each keyed template it leaves is read from the VM's strings
//! here, as a tooltip line reads it.

use mlua::Lua;

use crate::script::live::{ask, LiveQuery};
use crate::script::TooltipText;

/// The spell's description, its tokens expanded with no caster; `None` for no such spell or an
/// empty description.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpellDescriptionQuery(pub u32);

impl LiveQuery for SpellDescriptionQuery {
    type Answer = Option<TooltipText>;
}

/// The description of spell `id` now, nil-worthy as `None`; no host answers nothing.
pub fn spell_description(lua: &Lua, id: u32) -> Option<String> {
    ask(lua, &SpellDescriptionQuery(id)).flatten()?.line(lua)
}
