//! The host's half of ClassicAPI's `C_Spell.GetSpellDescription`
//! ([`benilla_ui::script::pfui_host::spell_description`]): a spell's description with its
//! `$`-tokens expanded outside a unit context, at the record's base values, as ClassicAPI
//! documents it: level 0, no spell modifiers, no caster's cost, gender 0, no home inn.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use benilla_ui::script::pfui_host::spell_description::SpellDescriptionQuery;
use benilla_ui::script::TooltipText;

#[derive(SystemParam)]
pub(crate) struct SpellTextLive<'w> {
    spells: Option<Res<'w, crate::ui_action::Spells>>,
}

impl SpellTextLive<'_> {
    fn describe(&self, id: u32) -> Option<TooltipText> {
        let spells = self.spells.as_deref()?;
        let d = spells.catalog.get(id)?;
        let text = d.description.as_deref().filter(|t| !t.is_empty())?;
        let ctx = benilla_formats::TokenContext {
            durations: &spells.durations,
            radii: &spells.radii,
            ranges: Some(&spells.ranges),
            level: &|_| 0,
            lookup: &|i| spells.catalog.get(i),
            mods: None,
            power_cost: &|_| -1,
            power_per_second: &|d| {
                crate::spell::usable::per_second_cost(d, crate::spell::SpellLevel::Known(0))
            },
            unmodified_points: false,
            gender: &|| 0,
            home_area: &|| None,
            // Never read: the parts leave each template to the VM's read.
            global: &|_| None,
            printf: &crate::ui_script::token_printf,
        };
        let parts =
            benilla_formats::substitute_parts(text, d, &ctx, benilla_formats::ExpandAt::default());
        Some(crate::ui_tooltip::tooltip_text(parts)).filter(|t| !t.is_empty())
    }
}

impl crate::script_calls::live::LiveFamily for SpellTextLive<'_> {
    fn answer(&mut self, ask: &mut benilla_ui::script::live::LiveAsk<'_>) -> bool {
        ask.answer(|q: &SpellDescriptionQuery| Some(self.describe(q.0)))
    }
}
