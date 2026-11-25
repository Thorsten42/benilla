//! ClassicAPI's `UNIT_SPELLCAST_SENT` on the cast's report lane (`spell/CastEvents.cpp`,
//! `OnSend`): the player's spell send, readied where the reports are, so it fires inside the
//! script call that sent it, or at its place among the lanes.

use benilla_ui::script::UiScript;

use crate::ui_action::Spells;

/// The send's event: its spell's name and rank, its unit target, and the castGUID the pfUI host
/// mints and holds for the cast ([`benilla_ui::script::pfui_host::casts::sent`]).
pub(super) fn ready(
    script: &UiScript,
    spells: Option<&Spells>,
    spell_id: u32,
    target: Option<u64>,
) -> super::Ready {
    let def = spells.and_then(|s| s.catalog.get(spell_id));
    let name = def.map(|d| d.name.clone()).unwrap_or_default();
    let rank = def.and_then(|d| d.rank.clone()).unwrap_or_default();
    super::Ready::Event(
        "UNIT_SPELLCAST_SENT",
        benilla_ui::script::pfui_host::casts::sent(script.lua(), spell_id, target, name, rank),
    )
}
