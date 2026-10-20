use alcmute_core::{Config, Result};
pub fn from_keys(keys: &[u16]) -> Result<Option<String>> {
    let modifier = |key| matches!(key, 0x10..=0x12 | 0x5b..=0x5c | 0xa0..=0xa5);
    let mut base: Vec<u16> = keys.iter().copied().filter(|key| !modifier(*key)).collect();
    base.sort_unstable();
    base.dedup();
    if base.is_empty() {
        let parts = modifier_parts(keys);
        return if parts.len() >= 2 {
            Ok(Some(parts.join("+")))
        } else {
            Ok(None)
        };
    }
    let mut parts = modifier_parts(keys);
    for key in base {
        let code = match key {
            0x30..=0x39 => format!("Digit{}", char::from_u32(key as u32).unwrap()),
            0x41..=0x5a => format!("Key{}", char::from_u32(key as u32).unwrap()),
            0x60..=0x69 => format!("Numpad{}", key - 0x60),
            0x70..=0x87 => format!("F{}", key - 0x6f),
            _ => match key {
                0x08 => "Backspace",
                0x09 => "Tab",
                0x0d => "Enter",
                0x13 => "Pause",
                0x14 => "CapsLock",
                0x20 => "Space",
                0x21 => "PageUp",
                0x22 => "PageDown",
                0x23 => "End",
                0x24 => "Home",
                0x25 => "ArrowLeft",
                0x26 => "ArrowUp",
                0x27 => "ArrowRight",
                0x28 => "ArrowDown",
                0x2c => "PrintScreen",
                0x2d => "Insert",
                0x2e => "Delete",
                0x5d => "ContextMenu",
                0x6a => "NumpadMultiply",
                0x6b => "NumpadAdd",
                0x6c => "NumpadComma",
                0x6d => "NumpadSubtract",
                0x6e => "NumpadDecimal",
                0x6f => "NumpadDivide",
                0x90 => "NumLock",
                0x91 => "ScrollLock",
                0xba => "Semicolon",
                0xbb => "Equal",
                0xbc => "Comma",
                0xbd => "Minus",
                0xbe => "Period",
                0xbf => "Slash",
                0xc0 => "Backquote",
                0xdb => "BracketLeft",
                0xdc => "Backslash",
                0xdd => "BracketRight",
                0xde => "Quote",
                0xe2 => "IntlBackslash",
                _ => return Err("このキーには対応していません。別のキーを押してください".into()),
            }
            .into(),
        };
        parts.push(code);
    }
    let shortcut = parts.join("+");
    Config {
        shortcut: shortcut.clone(),
        ..Default::default()
    }
    .validate()?;
    Ok(Some(shortcut))
}
fn modifier_parts(keys: &[u16]) -> Vec<String> {
    let mut parts = Vec::new();
    for (name, generic, left, right) in [
        ("Ctrl", 0x11, 0xa2, 0xa3),
        ("Alt", 0x12, 0xa4, 0xa5),
        ("Shift", 0x10, 0xa0, 0xa1),
        ("Super", 0, 0x5b, 0x5c),
    ] {
        if keys.contains(&left) {
            parts.push(format!("Left{name}"));
        }
        if keys.contains(&right) {
            parts.push(format!("Right{name}"));
        }
        if !keys.contains(&left) && !keys.contains(&right) && keys.contains(&generic) {
            parts.push(name.into());
        }
    }
    parts
}
pub fn label(shortcut: &str) -> String {
    shortcut
        .split('+')
        .map(|part| {
            if part == "LeftSuper" {
                "Left Win"
            } else if part == "RightSuper" {
                "Right Win"
            } else if part == "LeftCtrl" {
                "Left Ctrl"
            } else if part == "RightCtrl" {
                "Right Ctrl"
            } else if part == "LeftAlt" {
                "Left Alt"
            } else if part == "RightAlt" {
                "Right Alt"
            } else if part == "LeftShift" {
                "Left Shift"
            } else if part == "RightShift" {
                "Right Shift"
            } else if part == "Super" {
                "Win"
            } else {
                part.strip_prefix("Key")
                    .or_else(|| part.strip_prefix("Digit"))
                    .unwrap_or(part)
            }
        })
        .collect::<Vec<_>>()
        .join(" + ")
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captures_left_and_right_modifiers_and_function_keys() {
        assert_eq!(
            from_keys(&[0x11, 0xa2, 0x12, 0xa5, 0x4d]).unwrap(),
            Some("LeftCtrl+RightAlt+KeyM".into())
        );
        assert_eq!(
            from_keys(&[0x10, 0xa1, 0x7a]).unwrap(),
            Some("RightShift+F11".into())
        );
        assert_eq!(from_keys(&[0x87]).unwrap(), Some("F24".into()));
        assert_eq!(
            from_keys(&[0xa3, 0x61]).unwrap(),
            Some("RightCtrl+Numpad1".into())
        );
        assert_eq!(
            from_keys(&[0x5c, 0x4b]).unwrap(),
            Some("RightSuper+KeyK".into())
        );
        assert_eq!(label("Ctrl+Alt+KeyM"), "Ctrl + Alt + M");
    }
    #[test]
    fn accepts_plain_keys_and_multiple_base_keys_without_requiring_modifiers() {
        assert_eq!(from_keys(&[0x11, 0xa2]).unwrap(), None);
        assert_eq!(from_keys(&[0x41]).unwrap(), Some("KeyA".into()));
        assert_eq!(from_keys(&[0x4c, 0x4b]).unwrap(), Some("KeyK+KeyL".into()));
        assert_eq!(
            from_keys(&[0x11, 0x41, 0x42]).unwrap(),
            Some("Ctrl+KeyA+KeyB".into())
        );
        assert!(from_keys(&[0x11, 0xff]).is_err());
    }
    #[test]
    fn records_both_sides_and_modifier_only_chords() {
        assert_eq!(
            from_keys(&[0xa3, 0xa5]).unwrap(),
            Some("RightCtrl+RightAlt".into())
        );
        assert_eq!(
            from_keys(&[0xa2, 0xa4]).unwrap(),
            Some("LeftCtrl+LeftAlt".into())
        );
        assert_eq!(
            from_keys(&[0xa2, 0xa3, 0x4d]).unwrap(),
            Some("LeftCtrl+RightCtrl+KeyM".into())
        );

        assert_eq!(label("RightCtrl+RightAlt"), "Right Ctrl + Right Alt");
    }
}
