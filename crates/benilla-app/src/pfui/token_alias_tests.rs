//! The extension's tokens (a guid string, `nameplateN`, `markN`) read the unit with the host lent,
//! as a plate's update reads it in play.

use crate::pfui::test_support::{self, ME};
use benilla_ui::script::{pfui_host::units, UiScript, UnitState};

const MOB: u64 = 0x64;

#[test]
fn a_guid_token_reads_its_unit_with_the_host_lent() {
    let (mut app, _rx) = test_support::fixture();
    let world = app.world_mut();
    let mob = world
        .spawn((
            crate::net::Guid(MOB),
            crate::net::ObjectStore(benilla_protocol::ObjectFields::from_pairs(&[
                (2, 0x9),
                (22, 50),
                (28, 120),
                (34, 9),
            ])),
        ))
        .id();
    world
        .resource_mut::<crate::net::GuidIndex>()
        .0
        .insert(MOB, mob);
    let state = UnitState {
        guid: MOB,
        exists: true,
        has_object: true,
        level: 9,
        health: 50,
        max_health: 120,
        ..Default::default()
    };
    units::set_unit_states(
        world.non_send_resource::<UiScript>().lua(),
        [(MOB, state, ME)],
    );
    // Neither selected nor interacted with: the token takes the alias path, not `target`/`npc`.
    test_support::run(
        &mut app,
        r#"
        local t = '0x0000000000000064'
        assert(UnitExists(t) == 1, 'exists')
        assert(UnitLevel(t) == 9, 'level ' .. tostring(UnitLevel(t)))
        assert(UnitHealthMax(t) == 120, 'max ' .. tostring(UnitHealthMax(t)))
        assert(UnitIsUnit(t, t) == 1, 'same')
        assert(UnitIsUnit(t, 'mouseover') == nil, 'mouseover is not the mob')
    "#,
    );
}
