//! Android's on-screen keyboard, up while a text field reads the keys. Its keys arrive as ordinary key events
//! (NativeActivity has no text connection), which `UiContext` already types.
//!
//! The show is explicit: winit's `set_ime_allowed` asks for an implicit one, which Android refuses while a
//! controller or a hardware keyboard is in use, the handheld's case.

#[cfg(target_os = "android")]
use std::sync::OnceLock;

#[cfg(target_os = "android")]
use winit::platform::android::activity::AndroidApp;

#[cfg(target_os = "android")]
static ANDROID_APP: OnceLock<AndroidApp> = OnceLock::new();

/// Kept from `android_main`, for the keyboard
#[cfg(target_os = "android")]
pub(crate) fn set_android_app(app: &AndroidApp) {
    let _ = ANDROID_APP.set(app.clone());
}

/// What the keyboard should do this frame
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Change {
    Show,
    Hide,
}

/// Whether the keyboard is up, as last asked
#[derive(Debug, Default)]
pub(crate) struct SoftKeyboard {
    shown: bool,
}

impl SoftKeyboard {
    /// After the UI is built: `typing`, a text field read the keys; `tapped`, the player tapped it (the keyboard
    /// comes back if they put it away)
    pub(crate) fn frame(&mut self, typing: bool, tapped: bool) {
        if let Some(change) = self.change(typing, tapped) {
            apply(change);
        }
    }

    fn change(&mut self, typing: bool, tapped: bool) -> Option<Change> {
        let change = match (self.shown, typing) {
            (false, true) => Change::Show,
            (true, true) if tapped => Change::Show,
            (true, false) => Change::Hide,
            _ => return None,
        };
        self.shown = typing;
        Some(change)
    }
}

#[cfg(target_os = "android")]
fn apply(change: Change) {
    let Some(app) = ANDROID_APP.get() else {
        return;
    };
    match change {
        Change::Show => app.show_soft_input(false),
        Change::Hide => app.hide_soft_input(false),
    }
}

#[cfg(not(target_os = "android"))]
fn apply(_: Change) {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shows_when_typing_starts_and_hides_when_it_stops() {
        let mut kb = SoftKeyboard::default();
        assert_eq!(kb.change(true, false), Some(Change::Show));
        assert_eq!(kb.change(true, false), None);
        assert_eq!(kb.change(false, false), Some(Change::Hide));
        assert_eq!(kb.change(false, false), None);
    }

    #[test]
    fn a_tap_on_the_field_shows_it_again() {
        let mut kb = SoftKeyboard::default();
        kb.change(true, false);
        assert_eq!(kb.change(true, true), Some(Change::Show));
    }
}
