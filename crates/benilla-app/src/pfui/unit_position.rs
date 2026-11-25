//! The host's half of ClassicAPI's `UnitPosition`
//! ([`benilla_ui::script::pfui_host::unit_position`]): a held unit's world position at the call,
//! the player's from the controller, any other's composed through its parents (a rider on a
//! transport), and the current map.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use benilla_ui::script::pfui_host::unit_position::{
    PlayerFacingQuery, PlayerIndoorsQuery, UnitPositionQuery,
};

#[derive(SystemParam)]
pub(crate) struct UnitPositionLive<'w, 's> {
    index: Option<Res<'w, crate::net::GuidIndex>>,
    stores: Query<'w, 's, &'static crate::net::ObjectStore>,
    me: Query<'w, 's, Entity, With<crate::net::SelfPlayer>>,
    player: Option<Res<'w, crate::player::Player>>,
    transforms: bevy::transform::helper::TransformHelper<'w, 's>,
    map: Option<Res<'w, benilla_world::world_map::CurrentMap>>,
    /// The player's `CWorld::IsEntityOutdoors`, the spell validator's place answer, which
    /// ClassicAPI's `IsIndoors` reads (`FUN_UNIT_IS_OUTDOORS`); optional, so a harness without it
    /// cannot take the whole live-reads family down.
    outdoors: Option<Res<'w, crate::player::PlayerOutdoors>>,
}

impl UnitPositionLive<'_, '_> {
    fn position(&self, guid: u64) -> Option<([f32; 3], u32)> {
        let entity = *self.index.as_deref()?.0.get(&guid)?;
        if !self.stores.get(entity).is_ok_and(|s| s.is_unit()) {
            return None;
        }
        let at = if self.me.single().ok() == Some(entity) {
            self.player.as_deref()?.pos
        } else {
            self.transforms
                .compute_global_transform(entity)
                .ok()?
                .translation()
        };
        Some((
            benilla_assets::coords::bevy_to_wow(at),
            self.map.as_deref()?.0,
        ))
    }
}

impl crate::script_calls::live::LiveFamily for UnitPositionLive<'_, '_> {
    fn answer(&mut self, ask: &mut benilla_ui::script::live::LiveAsk<'_>) -> bool {
        ask.answer(|q: &UnitPositionQuery| Some(self.position(q.0)))
            || ask.answer(|_: &PlayerFacingQuery| {
                // The facing is the wire orientation itself (`net::motion::wire_yaw`).
                Some(self.player.as_deref().map(|p| p.facing()))
            })
            || ask.answer(|_: &PlayerIndoorsQuery| {
                Some(
                    self.me
                        .single()
                        .ok()
                        .and(self.outdoors.as_deref())
                        .map(|outdoors| !outdoors.0),
                )
            })
    }
}
