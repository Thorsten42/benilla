//! Nampower's `DisenchantAll(itemIdOrName | quality, [includeSoulbound])`: Disenchant cast on every
//! matching bag item, one after another ([`benilla_pfui::nampower`] takes the call; pfUI's
//! `/disenchantall` makes it). Ported from grom's benilla-octo (`spell/disenchant.rs`), which
//! reads the run off the DLL's own strings: the argument grammar, the stop reasons and the chat
//! lines.
//!
//! - **The filter:** a number is an item ID; `greens`, `blues`, `purples` are the Uncommon, Rare
//!   and Epic qualities and combine with `|`; any other text is an item name. Soulbound items are
//!   skipped unless asked for.
//! - **A step:** the next matching item, bag 0 to 4 and slot by slot, is cast at as a player does
//!   it: Disenchant through the ladder, which arms the targeting cursor, then the bag click's own
//!   bind (`pick_item`), with every check that click makes. The chat says which item, and that
//!   moving cancels.
//! - **Its end:** the item leaving the bag is success, and the next step follows. An item still
//!   there [`STEP_TIMEOUT`] after the cast is a failure that stops the run, as moving (which
//!   cancels the cast) does. No item left stops it with "No more items to disenchant.". A cast the
//!   player started is waited for, never fought.
// Deviation: the DLL prints each step's line through a Lua chunk with the item's link; here the
// host posts it as a system line with the item's name in brackets, not a clickable link.

use std::time::{Duration, Instant};

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

/// Disenchant's spell ID.
pub(crate) const DISENCHANT: u32 = 13_262;

/// How long a step waits for its item to leave the bag: the 3 s cast, the server's round trip and
/// slack. Too short stops a run on a laggy step.
const STEP_TIMEOUT: Duration = Duration::from_secs(10);

/// What a run takes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Filter {
    ItemId(u32),
    /// Lower-cased, matched against the lower-cased item name.
    Name(String),
    Qualities(Vec<u32>),
}

impl Filter {
    /// The DLL's grammar: a number is an ID, the three keywords (pipe-combined) are qualities, any
    /// other text a name; a keyword mixed with anything else is a name too.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        if let Ok(id) = text.trim().parse::<u32>() {
            return (id != 0).then_some(Filter::ItemId(id));
        }
        let qualities: Option<Vec<u32>> = text
            .split('|')
            .map(|part| match part.trim().to_ascii_lowercase().as_str() {
                "greens" => Some(2),
                "blues" => Some(3),
                "purples" => Some(4),
                _ => None,
            })
            .collect();
        match qualities {
            Some(q) if !q.is_empty() => Some(Filter::Qualities(q)),
            _ => (!text.trim().is_empty()).then(|| Filter::Name(text.trim().to_lowercase())),
        }
    }

    pub(crate) fn takes(&self, entry: u32, quality: u32, name: &str) -> bool {
        match self {
            Filter::ItemId(id) => entry == *id,
            Filter::Name(want) => name.to_lowercase() == *want,
            Filter::Qualities(set) => set.contains(&quality),
        }
    }
}

/// A run in progress.
#[derive(Resource, Default)]
pub(crate) struct DisenchantRun {
    filter: Option<Filter>,
    include_soulbound: bool,
    /// The item the current step cast at, and when it stops waiting for it to leave.
    step: Option<(u64, Instant)>,
}

/// What the driver does this frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Step {
    Idle,
    Waiting,
    /// Cast at this item, in bag `bag` and Lua slot `slot`.
    Cast {
        guid: u64,
        bag: i64,
        slot: u32,
    },
    /// Stop, with the chat line to show.
    Stop(&'static str),
}

pub(crate) const CHAT_NO_MORE: &str = "No more items to disenchant.";
pub(crate) const CHAT_FAILED: &str = "Disenchant interrupted or failed.";

impl DisenchantRun {
    /// Start, or restart with a new filter: a second call replaces the running one.
    pub(crate) fn start(&mut self, filter: Filter, include_soulbound: bool) {
        self.filter = Some(filter);
        self.include_soulbound = include_soulbound;
        self.step = None;
    }

    pub(crate) fn running(&self) -> bool {
        self.filter.is_some()
    }

    pub(crate) fn stop(&mut self) {
        self.filter = None;
        self.step = None;
    }

    /// The step sent at `guid` now.
    pub(crate) fn armed(&mut self, guid: u64, now: Instant) {
        self.step = Some((guid, now + STEP_TIMEOUT));
    }

    /// This frame's step. `still_there` answers whether an item is still in the bags; `next`
    /// finds the next matching item.
    pub(crate) fn advance(
        &mut self,
        now: Instant,
        in_flight: bool,
        still_there: impl Fn(u64) -> bool,
        next: impl Fn(&Filter, bool) -> Option<(u64, i64, u32)>,
    ) -> Step {
        let Some(filter) = self.filter.clone() else {
            return Step::Idle;
        };
        if let Some((guid, deadline)) = self.step {
            if in_flight {
                return Step::Waiting;
            }
            if still_there(guid) {
                if now < deadline {
                    return Step::Waiting;
                }
                self.stop();
                return Step::Stop(CHAT_FAILED);
            }
            self.step = None;
        } else if in_flight {
            return Step::Waiting;
        }
        match next(&filter, self.include_soulbound) {
            Some((guid, bag, slot)) => Step::Cast { guid, bag, slot },
            None => {
                self.stop();
                Step::Stop(CHAT_NO_MORE)
            }
        }
    }
}

/// The host state a step reads and the cast it sends.
#[derive(SystemParam)]
pub(crate) struct DisenchantDrive<'w, 's> {
    run: ResMut<'w, DisenchantRun>,
    targeting: crate::spell::cast_target::CastTargeting<'w, 's>,
    ladder: crate::spell::CastLadder<'w, 's>,
    enchants: Option<Res<'w, crate::items::Enchants>>,
    parked: ResMut<'w, crate::spell::targeting::EnchantConfirmItem>,
    chat: Res<'w, crate::ui_chat::ChatLog>,
}

impl DisenchantDrive<'_, '_> {
    fn say(&self, text: String) {
        self.chat.push_display(crate::ui_chat::ChatEvent::text_only(
            crate::ui_chat::ChatEventKind::System,
            text,
        ));
    }

    /// Take the runs the VM asked for, then one step.
    fn drive(&mut self, asks: Vec<(String, bool)>) {
        for (filter, include_soulbound) in asks {
            match Filter::parse(&filter) {
                Some(filter) => self.run.start(filter, include_soulbound),
                None => debug!("disenchant: {filter:?} names nothing; not started"),
            }
        }
        if !self.run.running() {
            return;
        }
        let now = Instant::now();
        let Some(store) = self.ladder.objects.player_fields() else {
            self.run.stop();
            return;
        };
        let objects = &self.ladder.objects;
        let in_bags = |slot_guid: &dyn Fn(i64, u8) -> Option<u64>, guid: u64| {
            (0i64..=4).any(|bag| (0u8..36).any(|slot| slot_guid(bag, slot) == Some(guid)))
        };
        let slot_guid = |bag: i64, slot: u8| crate::ui_items::slot_guid(store, bag, slot, objects);
        let enchants = self.enchants.as_deref();
        let items = &self.ladder.items;
        let next = |filter: &Filter, include_soulbound: bool| {
            (0i64..=4).find_map(|bag| {
                (0u8..36).find_map(|slot| {
                    let guid = slot_guid(bag, slot)?;
                    let fields = objects.object(guid)?;
                    let entry = fields.object_entry()?;
                    // A record still on its way is skipped this frame, not refused.
                    let t = items.template(entry, guid, crate::query_cache::Ask::Read)?;
                    if !include_soulbound && crate::items::already_bound(fields, t.flags, enchants)
                    {
                        return None;
                    }
                    filter
                        .takes(entry, t.quality, &t.name)
                        .then(|| (guid, bag, u32::from(slot) + 1))
                })
            })
        };
        let in_flight = self.ladder.pending.current().is_some();
        let step = self
            .run
            .advance(now, in_flight, |g| in_bags(&slot_guid, g), next);
        match step {
            Step::Idle | Step::Waiting => {}
            Step::Stop(line) => self.say(line.to_string()),
            Step::Cast { guid, bag, slot } => {
                let name = objects
                    .object(guid)
                    .and_then(|f| f.object_entry())
                    .and_then(|e| items.template(e, guid, crate::query_cache::Ask::Read))
                    .map(|t| t.name.clone())
                    .unwrap_or_default();
                self.say(format!(
                    "Disenchanting [{name}] move during cast to cancel."
                ));
                debug!("disenchant: casting at item {guid:#x} (bag {bag}, slot {slot})");
                // As the player does it: the spell arms the item cursor through the ladder, then
                // the bag click binds the item with every check that click makes.
                self.ladder.send(
                    DISENCHANT,
                    &self.targeting.context(),
                    crate::spell::CastCommit::Spell,
                );
                let _ = crate::spell::targeting::pick_item(
                    &mut self.ladder,
                    self.enchants.as_deref(),
                    &mut self.parked,
                    benilla_ui::script::ItemPick {
                        bag,
                        slot,
                        may_cast: true,
                    },
                );
                self.run.armed(guid, now);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<DisenchantRun>().add_systems(
        Update,
        // Among the frame's other engine casts: after the arrival's and the melee probe's.
        drive_disenchant
            .in_set(crate::target::EngineCasts)
            .after(benilla_world::terrain_stream::AreaAuthoritySet)
            .run_if(crate::ui_script::ingame_ui_up),
    );
}

/// Nampower's sequencer, once a frame: the runs `DisenchantAll` asked for, then a step.
pub(crate) fn drive_disenchant(
    mut vm: crate::script_calls::VmWith<DisenchantDrive<'static, 'static>>,
) {
    vm.turn(|script, mut drive| {
        let asks = script
            .map(|s| benilla_pfui::nampower::take_disenchant_asks(s.lua()))
            .unwrap_or_default();
        drive.drive(asks);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_filter_grammar_is_the_dlls() {
        assert_eq!(Filter::parse("12345"), Some(Filter::ItemId(12_345)));
        assert_eq!(Filter::parse("0"), None);
        assert_eq!(Filter::parse("GREENS"), Some(Filter::Qualities(vec![2])));
        assert_eq!(
            Filter::parse("greens | blues|purples"),
            Some(Filter::Qualities(vec![2, 3, 4]))
        );
        assert_eq!(
            Filter::parse("Linen Cloth"),
            Some(Filter::Name("linen cloth".into()))
        );
        assert_eq!(
            Filter::parse("greens|Linen Cloth"),
            Some(Filter::Name("greens|linen cloth".into()))
        );
        assert_eq!(Filter::parse("  "), None);
        assert!(Filter::Qualities(vec![2, 4]).takes(1, 4, "x"));
        assert!(!Filter::Qualities(vec![2, 4]).takes(1, 3, "x"));
        assert!(Filter::Name("linen cloth".into()).takes(2589, 1, "Linen Cloth"));
    }

    #[test]
    fn a_step_ends_when_its_item_leaves_and_a_stuck_one_stops_the_run() {
        let t0 = Instant::now();
        let mut run = DisenchantRun::default();
        assert_eq!(run.advance(t0, false, |_| true, |_, _| None), Step::Idle);
        let items = std::cell::RefCell::new(vec![(0xa1u64, 0i64, 1u32), (0xa2, 0, 2)]);
        let next = |_: &Filter, _: bool| items.borrow().first().copied();
        run.start(Filter::Qualities(vec![2]), false);
        assert_eq!(
            run.advance(t0, false, |_| true, next),
            Step::Cast {
                guid: 0xa1,
                bag: 0,
                slot: 1
            }
        );
        run.armed(0xa1, t0);
        assert_eq!(
            run.advance(t0, true, |_| true, next),
            Step::Waiting,
            "mid-cast"
        );
        assert_eq!(
            run.advance(t0, false, |_| true, next),
            Step::Waiting,
            "removal in flight"
        );
        items.borrow_mut().remove(0);
        assert_eq!(
            run.advance(t0, false, |g| g != 0xa1, next),
            Step::Cast {
                guid: 0xa2,
                bag: 0,
                slot: 2
            }
        );
        run.armed(0xa2, t0);
        items.borrow_mut().remove(0);
        assert_eq!(
            run.advance(t0, false, |_| false, next),
            Step::Stop(CHAT_NO_MORE)
        );
        assert!(!run.running());

        run.start(Filter::ItemId(7), false);
        let one = |_: &Filter, _: bool| Some((0xb1u64, 0i64, 3u32));
        assert!(matches!(
            run.advance(t0, false, |_| true, one),
            Step::Cast { .. }
        ));
        run.armed(0xb1, t0);
        assert_eq!(
            run.advance(t0 + STEP_TIMEOUT, false, |_| true, one),
            Step::Stop(CHAT_FAILED)
        );
        assert!(!run.running());
    }

    #[test]
    fn a_run_waits_for_a_cast_it_did_not_start() {
        let mut run = DisenchantRun::default();
        run.start(Filter::ItemId(7), false);
        assert_eq!(
            run.advance(Instant::now(), true, |_| false, |_, _| Some((0xc1, 0, 1))),
            Step::Waiting
        );
    }
}
