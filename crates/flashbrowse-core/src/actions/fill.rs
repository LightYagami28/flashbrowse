//! Fill / select-all key events.
//!
//! CDP modifier bitfield: `Alt=1, Ctrl=2, Meta/Command=4, Shift=8`.
//! macOS uses Cmd (Meta), Linux/Windows use Ctrl.

use flashbrowse_cdp::protocol::input::DispatchKeyEventParams;

/// Select-all modifier: Cmd on macOS, Ctrl elsewhere.
#[cfg(target_os = "macos")]
pub const MODIFIER_CTRL: i64 = 4;
#[cfg(not(target_os = "macos"))]
pub const MODIFIER_CTRL: i64 = 2;

/// `Cmd`/`Ctrl` + `A` key-down event (clear-before-type).
pub fn select_all_event() -> DispatchKeyEventParams {
    DispatchKeyEventParams {
        type_: "keyDown".to_string(),
        key: "a".to_string(),
        code: Some("KeyA".to_string()),
        modifiers: Some(MODIFIER_CTRL),
        windows_virtual_key_code: Some(65),
        native_virtual_key_code: None,
    }
}

/// `Backspace` key-down event (fires after select-all).
pub fn backspace_event() -> DispatchKeyEventParams {
    DispatchKeyEventParams {
        type_: "keyDown".to_string(),
        key: "Backspace".to_string(),
        code: Some("Backspace".to_string()),
        modifiers: None,
        windows_virtual_key_code: Some(8),
        native_virtual_key_code: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{backspace_event, select_all_event, MODIFIER_CTRL};

    #[test]
    fn select_all_has_correct_modifier() {
        let ev = select_all_event();
        assert_eq!(ev.key, "a");
        assert_eq!(ev.code.as_deref(), Some("KeyA"));
        assert_eq!(ev.modifiers, Some(MODIFIER_CTRL));
        // 2 = Ctrl (Linux/Windows), 4 = Meta/Cmd (macOS). Never Shift (8).
        assert!(ev.modifiers == Some(2) || ev.modifiers == Some(4));
        assert_ne!(ev.modifiers, Some(8));
    }

    #[test]
    fn backspace_vk() {
        assert_eq!(backspace_event().windows_virtual_key_code, Some(8));
    }
}
