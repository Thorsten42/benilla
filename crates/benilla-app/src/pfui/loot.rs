//! Additional packet observers preserve the stock roll UI and feed the optional history VM.

use crate::net::NetHandlerApp;
use benilla_pfui::loot::{self, Event, Key, Player};
use benilla_protocol::{SessionEvent, SessionEventKind};
use benilla_ui::script::UiScript;
use bevy::prelude::*;

/// Rolls whose item record had not landed at their start, waiting to learn whether it binds on
/// pickup.
#[derive(Resource, Default)]
struct AwaitingBonding(Vec<(Key, u32)>);

pub(super) fn register(app: &mut App) {
    app.init_resource::<AwaitingBonding>();
    app.add_systems(Update, bonding_landed.in_set(super::InputSnapshots));
    app.net_handler(SessionEventKind::LootStartRoll, receive);
    app.net_handler(SessionEventKind::LootRoll, receive);
    app.net_handler(SessionEventKind::LootRollWon, receive);
    app.net_handler(SessionEventKind::LootAllPassed, receive);
}

fn receive(
    In(event): In<SessionEvent>,
    script: Option<NonSend<UiScript>>,
    names: Res<crate::names::NameCache>,
    objects: crate::net::Objects,
    commands: Res<crate::net::NetCommands>,
    me: Res<crate::net::SelfGuid>,
    clock: Res<crate::ui_script::UiClock>,
    self_q: Query<&crate::net::ObjectStore, With<crate::net::SelfPlayer>>,
    items: Res<crate::items::Items>,
    mut awaiting: ResMut<AwaitingBonding>,
) {
    let Some(script) = script else { return };
    let player = |guid, kind, roll| {
        let class = names
            .player_traits(guid)
            .map(|(_, class, _)| class)
            .or_else(|| {
                (me.0 == Some(guid))
                    .then(|| self_q.single().ok()?.0.unit_class())
                    .flatten()
            });
        Player {
            guid,
            kind,
            roll,
            is_me: me.0 == Some(guid),
            name: names
                .resolve(&objects, guid, crate::query_cache::Ask::New(&commands))
                .map(str::to_owned),
            class: class
                .and_then(crate::pfui::class_names)
                .map(|(_, token)| token.to_owned()),
        }
    };
    let (key, id, suffix, event) = match event {
        SessionEvent::LootStartRoll(p) => (
            Key {
                target: p.looted_target,
                slot: p.item_slot,
            },
            p.item_id,
            p.random_property_id,
            Event::Start,
        ),
        SessionEvent::LootRoll(p) => {
            let Some(kind) = p.vote().or_else(|| p.is_dice().then_some(p.roll_type)) else {
                return;
            };
            (
                Key {
                    target: p.looted_target,
                    slot: p.item_slot,
                },
                p.item_id,
                p.random_property_id,
                Event::Roll(player(
                    p.roller,
                    kind,
                    if p.is_dice() { p.roll_number } else { 0 },
                )),
            )
        }
        SessionEvent::LootRollWon(p) => (
            Key {
                target: p.looted_target,
                slot: p.item_slot,
            },
            p.item_id,
            p.random_property_id,
            Event::Won(player(p.winner, p.roll_type, p.roll_number)),
        ),
        SessionEvent::LootAllPassed(p) => (
            Key {
                target: p.looted_target,
                slot: p.item_slot,
            },
            p.item_id,
            p.random_property_id,
            Event::Passed,
        ),
        _ => return,
    };
    let timestamp = clock.ui_now
        + bevy::platform::time::Instant::now()
            .duration_since(clock.anchor)
            .as_secs_f64();
    let start = matches!(event, Event::Start);
    loot::observe(script.lua(), key, id, suffix as i32, timestamp, event);
    // ClassicAPI's auto-show for a bind-on-pickup item, once its record says so.
    if start {
        match items.template(id, 0, crate::query_cache::Ask::Read) {
            Some(info) if info.bonding == 1 => loot::auto_show(script.lua(), key),
            Some(_) => {}
            None => awaiting.0.push((key, id)),
        }
    }
}

/// A roll waiting on its item's record shows once the record lands binding on pickup.
fn bonding_landed(
    script: Option<NonSend<UiScript>>,
    items: Res<crate::items::Items>,
    mut awaiting: ResMut<AwaitingBonding>,
) {
    let Some(script) = script else { return };
    if awaiting.0.is_empty() {
        return;
    }
    awaiting.0.retain(
        |&(key, id)| match items.template(id, 0, crate::query_cache::Ask::Read) {
            Some(info) => {
                if info.bonding == 1 {
                    loot::auto_show(script.lua(), key);
                }
                false
            }
            None => items.template_known(id),
        },
    );
}
