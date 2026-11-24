//! The gossip getters' reads at the call, the call host's half of
//! [`benilla_ui::script::GossipQuery`]: the message's slots and the drawn text as the handlers and
//! the selects left them (`0xbbbe90`, `0xbb74c0`, `0xbbb678`), which a close leaves for the getters
//! (`0x4e1fa0`).

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use benilla_ui::script::live::LiveAsk;
use benilla_ui::script::GossipQuery;

use super::GossipState;

/// What the gossip getters read as they run; the store absent in a harness without the gossip
/// window, where the getters read the VM's copy.
#[derive(SystemParam)]
pub(crate) struct GossipLive<'w> {
    state: Option<Res<'w, GossipState>>,
}

impl crate::script_calls::live::LiveFamily for GossipLive<'_> {
    fn answer(&mut self, ask: &mut LiveAsk<'_>) -> bool {
        #[cfg(feature = "pfui")]
        if ask.answer(
            |_: &benilla_ui::script::pfui_host::gossip_info::GossipInfoQuery| {
                Some(self.state.as_deref().and_then(info))
            },
        ) {
            return true;
        }
        ask.answer(|_: &GossipQuery| self.state.as_deref().map(super::menu))
    }
}

/// ClassicAPI's `C_GossipInfo` rows: the window's wire option indices, icons and quest ids, while
/// a window is live (opened, its text no longer pending).
#[cfg(feature = "pfui")]
fn info(state: &GossipState) -> Option<benilla_ui::script::pfui_host::gossip_info::GossipInfo> {
    (state.npc.is_some() && !state.pending).then(|| {
        benilla_ui::script::pfui_host::gossip_info::GossipInfo {
            options: state
                .options
                .iter()
                .filter(|o| (o.index as i32) >= 0)
                .map(|o| (o.index, o.icon, o.message.clone()))
                .collect(),
            quests: state.quests.clone(),
        }
    })
}
