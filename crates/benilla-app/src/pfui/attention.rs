//! The VM's `FlashClientIcon` asks, answered on the primary window
//! ([`benilla_ui::script::pfui_host::attention`]).

use benilla_ui::script::UiScript;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, answer.in_set(super::InputSnapshots));
}

/// `Informational`: "something happened while you were away", not a demand that keeps going until
/// the window is focused. A headless run has no winit window and drops the ask.
fn answer(script: Option<NonSend<UiScript>>, primary: Query<Entity, With<PrimaryWindow>>) {
    let Some(script) = script else { return };
    if !benilla_ui::script::pfui_host::attention::take(script.lua()) {
        return;
    }
    let Ok(entity) = primary.single() else { return };
    bevy::winit::WINIT_WINDOWS.with_borrow(|windows| {
        if let Some(window) = windows.get_window(entity) {
            window.request_user_attention(Some(winit::window::UserAttentionType::Informational));
        }
    });
}
