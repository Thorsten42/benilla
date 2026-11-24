//! ClassicAPI's `WEAPON_ENCHANT_CHANGED` and `WEAPON_SLOT_CHANGED`, no payload (`item/
//! WeaponEnchant.cpp`, `player/Equipment.cpp`): retail's "read the weapon enchants again", which
//! pfUI's buff frame refreshes its weapon buttons on instead of polling.
//!
//! 1.12 signals neither. ClassicAPI marks the enchant event where a temporary enchant on the
//! main hand, off hand or ranged weapon changes: the enchant-time update (a renewal moves only
//! the item's expiry clock) and the item's enchantment field (applied, removed, a charge spent).
//! Each fires at most once a frame, on the next world tick. Here the frame's pass compares the
//! three weapons with the last pass: a new item in a slot is `WEAPON_SLOT_CHANGED`; an enchant
//! that came, went, changed its id or charges, or whose time left went up (a renewal) is
//! `WEAPON_ENCHANT_CHANGED`. A clock running down changes nothing.

use benilla_ui::script::{pfui_host::fire_pfui_event, UiScript};
use bevy::prelude::*;

use crate::ui_items::ItemObjects;

/// Main hand, off hand and ranged, 0-based equipment slots.
const WEAPON_SLOTS: [u8; 3] = [15, 16, 17];
/// `ITEM_FIELD_ENCHANTMENT`'s temporary enchant.
const TEMP_ENCHANTMENT_SLOT: u8 = 1;

#[derive(Clone, Copy, Default, PartialEq)]
struct Weapon {
    item: Option<u64>,
    /// The temporary enchant's id and charges, `None` for none.
    enchant: Option<(u32, u32)>,
    remaining_ms: Option<u64>,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, feed.in_set(super::InputSnapshots));
}

fn weapons(objects: &crate::net::Objects) -> Option<[Weapon; 3]> {
    let me = objects.player_fields()?;
    Some(WEAPON_SLOTS.map(|slot| {
        let Some(item) = me.player_inv_slot(slot) else {
            return Weapon::default();
        };
        let enchant = ItemObjects::object(objects, item).and_then(|obj| {
            let id = obj.item_enchant(TEMP_ENCHANTMENT_SLOT)?;
            Some((id as u32, obj.item_enchant_charges(TEMP_ENCHANTMENT_SLOT)))
        });
        let remaining_ms = ItemObjects::countdowns(objects, item)
            .and_then(|c| c.enchant_deadline_ms(u32::from(TEMP_ENCHANTMENT_SLOT)));
        Weapon {
            item: Some(item),
            enchant,
            remaining_ms,
        }
    }))
}

/// Whether a weapon slot took another item, and whether a temporary enchant changed, from one
/// pass to the next.
fn edges(before: &[Weapon; 3], after: &[Weapon; 3]) -> (bool, bool) {
    let slot = before.iter().zip(after).any(|(b, a)| b.item != a.item);
    let enchant = before.iter().zip(after).any(|(b, a)| {
        (b.item != a.item && (b.enchant.is_some() || a.enchant.is_some()))
            || b.enchant != a.enchant
            || a.remaining_ms.unwrap_or(0) > b.remaining_ms.unwrap_or(0)
    });
    (slot, enchant)
}

fn feed(
    script: Option<NonSend<UiScript>>,
    objects: crate::net::Objects,
    mut last: Local<crate::ui_script::VmMemo<Option<[Weapon; 3]>>>,
) {
    let Some(script) = script else { return };
    let now = weapons(&objects);
    let last = last.get(&script);
    let (slot_changed, enchant_changed) = match (&*last, &now) {
        (Some(before), Some(after)) => edges(before, after),
        _ => (false, false),
    };
    *last = now;
    if slot_changed {
        fire_pfui_event(script.lua(), "WEAPON_SLOT_CHANGED", vec![]);
    }
    if enchant_changed {
        fire_pfui_event(script.lua(), "WEAPON_ENCHANT_CHANGED", vec![]);
    }
}

#[cfg(test)]
mod tests {
    use super::{edges, Weapon};

    fn sword(enchant: Option<(u32, u32)>, remaining_ms: Option<u64>) -> Weapon {
        Weapon {
            item: Some(7),
            enchant,
            remaining_ms,
        }
    }

    /// A clock running down is no change; a renewal (more time), a spent charge, an enchant that
    /// ends, and a new item are.
    #[test]
    fn only_a_real_change_fires() {
        let none = Weapon::default();
        let a = [sword(Some((29, 0)), Some(60_000)), none, none];
        let ticked = [sword(Some((29, 0)), Some(59_000)), none, none];
        assert_eq!(edges(&a, &ticked), (false, false));
        let renewed = [sword(Some((29, 0)), Some(1_800_000)), none, none];
        assert_eq!(edges(&ticked, &renewed), (false, true));
        let charge = [sword(Some((29, 4)), Some(59_000)), none, none];
        assert_eq!(
            edges(&[sword(Some((29, 5)), Some(59_000)), none, none], &charge),
            (false, true)
        );
        let gone = [sword(None, None), none, none];
        assert_eq!(edges(&ticked, &gone), (false, true));
        let swapped = [
            Weapon {
                item: Some(8),
                ..none
            },
            none,
            none,
        ];
        assert_eq!(edges(&gone, &swapped), (true, false));
        assert_eq!(
            edges(&ticked, &swapped),
            (true, true),
            "the enchanted weapon left"
        );
    }
}
