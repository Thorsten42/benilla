//! The host half of ClassicAPI's `C_Loot` corpse walk (`loot/Scan.cpp`, `Nearby.cpp`,
//! `Request.cpp`; the VM half is [`benilla_ui::script::pfui_host::area_loot`]).
//!
//! Each frame it feeds the VM the lootable corpses in interact range: units the server flagged
//! lootable for us (`UNIT_DYNFLAG_LOOTABLE` 0x1), inside the melee reach `CanLootNow` measures
//! (`max(rA + rB + 1.333, 5)`). A walk (`LootAllCorpses`, `ScanNearbyLoot`) opens each corpse in
//! turn with `CMSG_LOOT` and claims its `SMSG_LOOT_RESPONSE` before the loot window's handler
//! sees it ([`claim`]): no select, no latch, no loot frame, no `LOOT_OPENED`/`LOOT_CLOSED`. It
//! records the corpse; a loot walk takes the coin and every row it may take (ALLOW_LOOT, OWNER),
//! then releases and goes on. A corpse silent for 3 s is released and left out; one that opened
//! with nothing for us but stays lootable is skipped by later walks until it leaves view. Each
//! walk ends with `LOOT_SCAN_COMPLETED`. `LootUnitItem` is a one-corpse walk that takes the one
//! item.
// Deviation: ClassicAPI also forgets a skipped corpse when the quest log changes; here only
// leaving view does. A walk asked while a loot window is open ends at once with no corpse.

use std::collections::{HashSet, VecDeque};
use std::time::{Duration, Instant};

use benilla_protocol::messages::slot_type;
use benilla_protocol::messages::LootItem;
use benilla_ui::script::pfui_host::area_loot::{self, LootRequest, ScannedCorpse};
use benilla_ui::script::UiScript;
use bevy::prelude::*;

use crate::net::{ClientCommand, NetCommands};

/// How long a corpse may stay silent before the walk leaves it (ClassicAPI: 180 frames).
const SILENT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Take {
    /// Read the window, take nothing.
    Nothing,
    /// The coin and every row we may take.
    All,
    /// The rows of one item.
    Item(u32),
}

struct Current {
    guid: u64,
    since: Instant,
    answered: bool,
}

#[derive(Resource, Default)]
pub(crate) struct AreaLootWalk {
    queue: VecDeque<u64>,
    take: Option<Take>,
    current: Option<Current>,
    results: Vec<ScannedCorpse>,
    /// Corpses a loot walk found nothing for us on, skipped until they leave view.
    skipped: HashSet<u64>,
    /// Corpses the walk released, whose release answer is the walk's.
    released: HashSet<u64>,
    /// A walk ended and owes the VM its results and `LOOT_SCAN_COMPLETED`.
    finished: bool,
}

impl AreaLootWalk {
    fn running(&self) -> bool {
        self.take.is_some()
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<AreaLootWalk>();
    app.add_systems(
        Update,
        feed.in_set(super::InputSnapshots)
            .run_if(crate::ui_script::ingame_ui_up),
    );
}

/// The response for the corpse the walk opened, which the walk takes in the loot window's
/// place; whether it was the walk's.
pub(crate) fn claim(
    walk: &mut AreaLootWalk,
    net: &NetCommands,
    guid: u64,
    gold: u32,
    rows: &[LootItem],
) -> bool {
    let Some(take) = walk.take else {
        return false;
    };
    let Some(current) = walk
        .current
        .as_mut()
        .filter(|c| c.guid == guid && !c.answered)
    else {
        return false;
    };
    current.answered = true;
    let takeable =
        |row: &&LootItem| matches!(row.slot_type, slot_type::ALLOW_LOOT | slot_type::OWNER);
    let mut corpse = ScannedCorpse {
        guid,
        coin: gold,
        items: Vec::new(),
    };
    for row in rows {
        corpse.items.push((row.item_id, row.count, None));
    }
    match take {
        Take::Nothing => {}
        Take::All => {
            if gold > 0 {
                let _ = net.0.send(ClientCommand::LootMoney);
            }
            let mut any = gold > 0;
            for row in rows.iter().filter(takeable) {
                let _ = net
                    .0
                    .send(ClientCommand::AutostoreLootItem { slot: row.slot });
                any = true;
            }
            if !any {
                walk.skipped.insert(guid);
            }
        }
        Take::Item(item_id) => {
            for row in rows
                .iter()
                .filter(takeable)
                .filter(|r| r.item_id == item_id)
            {
                let _ = net
                    .0
                    .send(ClientCommand::AutostoreLootItem { slot: row.slot });
            }
        }
    }
    let _ = net.0.send(ClientCommand::LootRelease { guid });
    walk.released.insert(guid);
    if !matches!(take, Take::Item(_)) {
        walk.results.push(corpse);
    }
    true
}

/// A release answer for a corpse the walk released, which no window waits for.
pub(crate) fn claim_release(walk: &mut AreaLootWalk, guid: u64) -> bool {
    walk.released.remove(&guid)
}

#[allow(clippy::too_many_arguments)] // a Bevy system's inputs
fn feed(
    script: Option<NonSend<UiScript>>,
    mut walk: ResMut<AreaLootWalk>,
    net: Res<NetCommands>,
    loot: Res<crate::ui_loot::LootState>,
    me: Query<(&crate::net::ObjectStore, &Transform), With<crate::net::SelfPlayer>>,
    units: Query<(&crate::net::Guid, &crate::net::ObjectStore, &Transform)>,
) {
    let Some(script) = script else { return };
    let lua = script.lua();
    let now = Instant::now();
    // The lootable corpses in reach, and every lootable one in view for the skip list.
    let mut in_view = HashSet::new();
    let mut nearby = Vec::new();
    if let Ok((my_store, my_place)) = me.single() {
        for (guid, store, place) in &units {
            if !store.is_unit() || store.0.unit_dynamic_flags() & 0x1 == 0 {
                continue;
            }
            in_view.insert(guid.0);
            let reach = crate::target::cursor_mode::melee_reach(store, my_store);
            if place.translation.distance_squared(my_place.translation) <= reach * reach {
                nearby.push(guid.0);
            }
        }
    }
    nearby.sort_unstable();
    walk.skipped.retain(|guid| in_view.contains(guid));
    area_loot::set_nearby(lua, nearby.clone());
    for request in area_loot::take_requests(lua) {
        match request {
            LootRequest::Scan | LootRequest::LootAll { .. } if walk.running() => {}
            LootRequest::Scan | LootRequest::LootAll { .. } => {
                let (take, max) = match request {
                    LootRequest::LootAll { max } => (Take::All, max),
                    _ => (Take::Nothing, None),
                };
                walk.results.clear();
                walk.take = Some(take);
                walk.queue.clear();
                if loot.source().is_none() {
                    let skipped = walk.skipped.clone();
                    walk.queue.extend(
                        nearby
                            .iter()
                            .copied()
                            .filter(|g| take == Take::Nothing || !skipped.contains(g))
                            .take(max.map_or(usize::MAX, |m| m as usize)),
                    );
                }
            }
            LootRequest::UnitItem { guid, item_id } => {
                if !walk.running() && loot.source().is_none() {
                    walk.results.clear();
                    walk.take = Some(Take::Item(item_id));
                    walk.queue.clear();
                    walk.queue.push_back(guid);
                }
            }
        }
    }
    if walk.running() {
        let done = walk
            .current
            .as_ref()
            .is_none_or(|c| c.answered || now.duration_since(c.since) >= SILENT);
        if done {
            if let Some(silent) = walk.current.take().filter(|c| !c.answered) {
                debug!("area loot: {:#x} stayed silent, released", silent.guid);
                let _ = net.0.send(ClientCommand::LootRelease { guid: silent.guid });
                walk.released.insert(silent.guid);
            }
            match walk.queue.pop_front() {
                Some(guid) => {
                    let _ = net.0.send(ClientCommand::Loot { guid });
                    walk.current = Some(Current {
                        guid,
                        since: now,
                        answered: false,
                    });
                }
                None => {
                    let item_walk = matches!(walk.take, Some(Take::Item(_)));
                    walk.take = None;
                    walk.finished = !item_walk;
                    if item_walk {
                        area_loot::set_in_progress(lua, false);
                    }
                }
            }
        }
    }
    if std::mem::take(&mut walk.finished) {
        area_loot::set_results(lua, walk.results.clone());
        area_loot::set_in_progress(lua, false);
        benilla_ui::script::pfui_host::fire_pfui_event(lua, "LOOT_SCAN_COMPLETED", vec![]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(slot: u8, item_id: u32, slot_type: u8) -> LootItem {
        LootItem {
            slot,
            item_id,
            count: 1,
            display_info_id: 0,
            random_property_id: 0,
            slot_type,
        }
    }

    fn walking(take: Take, guid: u64) -> AreaLootWalk {
        AreaLootWalk {
            take: Some(take),
            current: Some(Current {
                guid,
                since: Instant::now(),
                answered: false,
            }),
            ..Default::default()
        }
    }

    /// A loot walk's claim takes the coin and every row it may take (ALLOW_LOOT, OWNER, not a
    /// roll in progress), then releases; a scan only reads and releases; a response for another
    /// corpse, or a second one, is the loot window's; a corpse with nothing for us is skipped.
    #[test]
    fn the_walk_takes_what_its_mode_asks() {
        let (tx, rx) = crossbeam_channel::unbounded();
        let net = NetCommands(tx);
        let mut walk = walking(Take::All, 7);
        let rows = [
            row(0, 2589, slot_type::ALLOW_LOOT),
            row(1, 3000, slot_type::ROLL_ONGOING),
            row(2, 4000, slot_type::OWNER),
        ];
        assert!(
            !claim(&mut walk, &net, 8, 50, &rows),
            "another corpse's window"
        );
        assert!(claim(&mut walk, &net, 7, 50, &rows));
        assert!(!claim(&mut walk, &net, 7, 50, &rows), "answered once");
        let sent: Vec<_> = rx.try_iter().collect();
        assert!(matches!(
            sent.as_slice(),
            [
                ClientCommand::LootMoney,
                ClientCommand::AutostoreLootItem { slot: 0 },
                ClientCommand::AutostoreLootItem { slot: 2 },
                ClientCommand::LootRelease { guid: 7 },
            ]
        ));
        assert_eq!(walk.results.len(), 1);
        assert_eq!(walk.results[0].coin, 50);
        assert_eq!(walk.results[0].items.len(), 3);
        assert!(claim_release(&mut walk, 7) && !claim_release(&mut walk, 7));

        let mut empty = walking(Take::All, 9);
        assert!(claim(
            &mut empty,
            &net,
            9,
            0,
            &[row(0, 3000, slot_type::LOCKED)]
        ));
        assert!(empty.skipped.contains(&9), "nothing for us: skipped");
        let _ = rx.try_iter().count();

        let mut scan = walking(Take::Nothing, 5);
        assert!(claim(&mut scan, &net, 5, 10, &rows));
        assert!(matches!(
            rx.try_iter().collect::<Vec<_>>().as_slice(),
            [ClientCommand::LootRelease { guid: 5 }]
        ));

        let mut item = walking(Take::Item(4000), 6);
        assert!(claim(&mut item, &net, 6, 10, &rows));
        assert!(matches!(
            rx.try_iter().collect::<Vec<_>>().as_slice(),
            [
                ClientCommand::AutostoreLootItem { slot: 2 },
                ClientCommand::LootRelease { guid: 6 },
            ]
        ));
        assert!(item.results.is_empty(), "an item take reports no scan");
        assert!(
            !claim(&mut AreaLootWalk::default(), &net, 6, 0, &rows),
            "no walk"
        );
    }
}
