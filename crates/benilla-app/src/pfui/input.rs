//! The ClassicAPI input the extension's VM sees beside 1.12's: `GLOBAL_MOUSE_DOWN` and
//! `GLOBAL_MOUSE_UP` for every button press and release, and the clipboard `CopyToClipboard` fills.

use benilla_ui::script::{ScriptValue, UiScript};
use bevy::prelude::*;
use bevy::window::{PrimaryWindow, RawHandleWrapper};

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        copy_to_clipboard
            .in_set(crate::ui_script::UiFeed)
            .run_if(crate::ui_script::ingame_ui_up),
    );
}

/// A press or release, named as the UI's handlers get it, before the frame under the cursor gets
/// its `OnMouseDown` or `OnMouseUp`.
pub(crate) fn global_mouse(script: &UiScript, button: &str, down: bool) {
    let event = if down {
        "GLOBAL_MOUSE_DOWN"
    } else {
        "GLOBAL_MOUSE_UP"
    };
    benilla_ui::script::pfui_host::fire_pfui_event(
        script.lua(),
        event,
        vec![ScriptValue::Str(button.to_string())],
    );
}

/// The text a script copied this frame, written as the EditBox's copy writes it.
fn copy_to_clipboard(
    script: Option<NonSend<UiScript>>,
    window: Query<Option<&RawHandleWrapper>, With<PrimaryWindow>>,
    mut clipboard: NonSendMut<crate::textinput::HostClipboard>,
) {
    let Some(script) = script else { return };
    let Some(text) = manila_pfui::take_clipboard(&script) else {
        return;
    };
    let handle = window.single().ok().flatten();
    clipboard.write(crate::textinput::wayland_display(handle), &text);
}
