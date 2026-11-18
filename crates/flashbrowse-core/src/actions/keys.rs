//! Key name -> CDP key event mapping.
//!
//! `Input.dispatchKeyEvent` wants the key *value* in `key` and the DOM
//! `KeyboardEvent.code` in `code` (e.g. "a" -> "KeyA"). Sending the key
//! name in `code` breaks layout-dependent keys.

/// A resolved key: value, DOM code and (optional) virtual key code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeySpec {
    /// `KeyboardEvent.key`
    pub key: String,
    /// `KeyboardEvent.code`
    pub code: String,
    /// `windowsVirtualKeyCode` (Windows/Linux)
    pub vk: Option<i64>,
}

/// Resolve a user-facing key name ("Enter", "a", "ArrowDown", ...).
pub fn resolve(key: &str) -> Option<KeySpec> {
    let k = key.trim();
    if k.is_empty() {
        return None;
    }
    let spec = match k {
        "Enter" | "Return" => KeySpec {
            key: "Enter".into(),
            code: "Enter".into(),
            vk: Some(13),
        },
        "Tab" => KeySpec {
            key: "Tab".into(),
            code: "Tab".into(),
            vk: Some(9),
        },
        "Escape" | "Esc" => KeySpec {
            key: "Escape".into(),
            code: "Escape".into(),
            vk: Some(27),
        },
        "Backspace" => KeySpec {
            key: "Backspace".into(),
            code: "Backspace".into(),
            vk: Some(8),
        },
        "Delete" | "Del" => KeySpec {
            key: "Delete".into(),
            code: "Delete".into(),
            vk: Some(46),
        },
        "Space" | " " => KeySpec {
            key: " ".into(),
            code: "Space".into(),
            vk: Some(32),
        },
        "ArrowUp" | "Up" => KeySpec {
            key: "ArrowUp".into(),
            code: "ArrowUp".into(),
            vk: Some(38),
        },
        "ArrowDown" | "Down" => KeySpec {
            key: "ArrowDown".into(),
            code: "ArrowDown".into(),
            vk: Some(40),
        },
        "ArrowLeft" | "Left" => KeySpec {
            key: "ArrowLeft".into(),
            code: "ArrowLeft".into(),
            vk: Some(37),
        },
        "ArrowRight" | "Right" => KeySpec {
            key: "ArrowRight".into(),
            code: "ArrowRight".into(),
            vk: Some(39),
        },
        "PageUp" => KeySpec {
            key: "PageUp".into(),
            code: "PageUp".into(),
            vk: Some(33),
        },
        "PageDown" => KeySpec {
            key: "PageDown".into(),
            code: "PageDown".into(),
            vk: Some(34),
        },
        "Home" => KeySpec {
            key: "Home".into(),
            code: "Home".into(),
            vk: Some(36),
        },
        "End" => KeySpec {
            key: "End".into(),
            code: "End".into(),
            vk: Some(35),
        },
        "Shift" => KeySpec {
            key: "Shift".into(),
            code: "ShiftLeft".into(),
            vk: Some(16),
        },
        "Control" | "Ctrl" => KeySpec {
            key: "Control".into(),
            code: "ControlLeft".into(),
            vk: Some(17),
        },
        "Alt" => KeySpec {
            key: "Alt".into(),
            code: "AltLeft".into(),
            vk: Some(18),
        },
        "Meta" | "Command" | "Cmd" => KeySpec {
            key: "Meta".into(),
            code: "MetaLeft".into(),
            vk: Some(91),
        },
        _ => {
            // Single printable character -> letter/digit layout codes.
            let mut chars = k.chars();
            let c = chars.next()?;
            if chars.next().is_some() {
                return None;
            }
            if c.is_ascii_alphabetic() {
                let upper = c.to_ascii_uppercase();
                KeySpec {
                    key: c.to_string(),
                    code: format!("Key{upper}"),
                    vk: Some(upper as i64),
                }
            } else if c.is_ascii_digit() {
                KeySpec {
                    key: c.to_string(),
                    code: format!("Digit{c}"),
                    vk: Some(c as i64),
                }
            } else {
                return None;
            }
        }
    };
    Some(spec)
}

#[cfg(test)]
mod tests {
    use super::resolve;

    #[test]
    fn named_keys() {
        let e = resolve("Enter").unwrap();
        assert_eq!(e.key, "Enter");
        assert_eq!(e.code, "Enter");
        assert_eq!(e.vk, Some(13));
    }

    #[test]
    fn letters_use_dom_code() {
        let a = resolve("a").unwrap();
        assert_eq!(a.code, "KeyA");
        assert_eq!(a.key, "a");
        assert_eq!(a.vk, Some(65));
        let z = resolve("Z").unwrap();
        assert_eq!(z.code, "KeyZ");
        assert_eq!(z.key, "Z");
    }

    #[test]
    fn digits_use_digit_code() {
        let d = resolve("7").unwrap();
        assert_eq!(d.code, "Digit7");
        assert_eq!(d.vk, Some(55));
    }

    #[test]
    fn rejects_empty_and_multi_char() {
        assert!(resolve("").is_none());
        assert!(resolve("   ").is_none());
        assert!(resolve("Ctrl+C").is_none());
    }
}
