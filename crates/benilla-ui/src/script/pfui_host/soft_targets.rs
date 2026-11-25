//! The VM half of ClassicAPI's soft targets (`target/SoftUnit.cpp`, `SoftInteract.cpp`): the
//! units the host picked ([`set`]) behind the tokens `softenemy`, `softfriend` and `softinteract`
//! and their `any…` forms (`anyenemy`, `anyfriend`, `anyinteract`; `anyleftinteract` is
//! `anyinteract`), each with a `target` chain; and the change events
//! `PLAYER_SOFT_ENEMY_CHANGED`, `PLAYER_SOFT_FRIEND_CHANGED` and
//! `PLAYER_SOFT_INTERACT_CHANGED(oldGUID, newGUID)`. With no host feed an `any…` token is the
//! target, as ClassicAPI's is while its soft unit is empty.

use mlua::Lua;

use crate::script::ScriptValue;

/// One frame's picks: the soft units and the `any…` units ClassicAPI's `GetAny` gives.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SoftPicks {
    pub enemy: Option<u64>,
    pub friend: Option<u64>,
    pub interact: Option<u64>,
    pub any_enemy: Option<u64>,
    pub any_friend: Option<u64>,
    pub any_interact: Option<u64>,
}

#[derive(Default)]
struct SoftTargets {
    picks: Option<SoftPicks>,
}

/// The host's picks now; each soft unit that changed fires its event.
pub fn set(lua: &Lua, picks: SoftPicks) {
    let old = {
        let Some(mut state) = lua.app_data_mut::<SoftTargets>() else {
            return;
        };
        let old = state.picks.unwrap_or_default();
        state.picks = Some(picks);
        old
    };
    if old.enemy != picks.enemy {
        super::fire_pfui_event(lua, "PLAYER_SOFT_ENEMY_CHANGED", vec![]);
    }
    if old.friend != picks.friend {
        super::fire_pfui_event(lua, "PLAYER_SOFT_FRIEND_CHANGED", vec![]);
    }
    if old.interact != picks.interact {
        let guid = |g: Option<u64>| {
            ScriptValue::Str(g.map(super::guid_events::guid_string).unwrap_or_default())
        };
        super::fire_pfui_event(
            lua,
            "PLAYER_SOFT_INTERACT_CHANGED",
            vec![guid(old.interact), guid(picks.interact)],
        );
    }
}

/// The unit a soft or `any…` token names (its base, before any `target` chain); `None` when
/// `token` is none of them.
pub(crate) fn base(lua: &Lua, token: &str) -> Option<Option<u64>> {
    let picks = lua.app_data_ref::<SoftTargets>().and_then(|s| s.picks);
    let target = || {
        lua.app_data_ref::<crate::script::Model>()
            .map(|m| m.unit_guids.target)
            .filter(|g| *g != 0)
    };
    let any = |pick: fn(&SoftPicks) -> Option<u64>| match picks {
        Some(p) => pick(&p),
        None => target(),
    };
    Some(match token {
        "softenemy" => picks.and_then(|p| p.enemy),
        "softfriend" => picks.and_then(|p| p.friend),
        "softinteract" => picks.and_then(|p| p.interact),
        "anyenemy" => any(|p| p.any_enemy),
        "anyfriend" => any(|p| p.any_friend),
        "anyinteract" | "anyleftinteract" => any(|p| p.any_interact),
        _ => return None,
    })
}

pub(super) fn install(lua: &Lua) {
    lua.set_app_data(SoftTargets::default());
}
