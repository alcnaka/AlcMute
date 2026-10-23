use crate::Result;
use std::{cell::RefCell, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HotKey(Chord);
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    modifiers: [u8; 4],
    keys: [u64; 4],
}
pub fn key_name_to_vk(name: &str) -> Option<u16> {
    if name.len() == 1 && name.as_bytes()[0].is_ascii_alphanumeric() {
        return Some(name.as_bytes()[0].to_ascii_uppercase() as u16);
    }
    if let Some(n) = name.strip_prefix('F').and_then(|s| s.parse::<u16>().ok()) {
        if (1..=24).contains(&n) {
            return Some(0x6f + n);
        }
    }
    for (prefix, start, end) in [("Key", 0x41, 0x5a), ("Digit", 0x30, 0x39)] {
        if let Some(s) = name.strip_prefix(prefix) {
            if s.len() == 1 {
                let v = s.as_bytes()[0] as u16;
                if (start..=end).contains(&v) {
                    return Some(v);
                }
            }
        }
    }
    if let Some(n) = name
        .strip_prefix("Numpad")
        .and_then(|s| s.parse::<u16>().ok())
    {
        if n <= 9 {
            return Some(0x60 + n);
        }
    }
    Some(match name {
        "Backspace" => 8,
        "Tab" => 9,
        "Enter" => 13,
        "NumpadEnter" => 13,
        "NumpadClear" => 12,
        "Pause" => 19,
        "CapsLock" => 20,
        "KanaMode" => 21,
        "Escape" => 27,
        "Convert" => 28,
        "NonConvert" => 29,
        "Space" => 32,
        "PageUp" => 33,
        "PageDown" => 34,
        "End" => 35,
        "Home" => 36,
        "ArrowLeft" => 37,
        "ArrowUp" => 38,
        "ArrowRight" => 39,
        "ArrowDown" => 40,
        "PrintScreen" => 44,
        "Insert" => 45,
        "Delete" => 46,
        "Help" => 47,
        "ContextMenu" => 93,
        "Sleep" => 95,
        "NumpadMultiply" => 106,
        "NumpadAdd" => 107,
        "NumpadComma" => 108,
        "NumpadSubtract" => 109,
        "NumpadDecimal" => 110,
        "NumpadDivide" => 111,
        "NumLock" => 144,
        "ScrollLock" => 145,
        "NumpadEqual" => 146,
        "BrowserBack" => 166,
        "BrowserForward" => 167,
        "BrowserRefresh" => 168,
        "BrowserStop" => 169,
        "BrowserSearch" => 170,
        "BrowserFavorites" => 171,
        "BrowserHome" => 172,
        "AudioVolumeMute" => 173,
        "AudioVolumeDown" => 174,
        "AudioVolumeUp" => 175,
        "MediaTrackNext" => 176,
        "MediaTrackPrevious" => 177,
        "MediaStop" => 178,
        "MediaPlayPause" => 179,
        "LaunchMail" => 180,
        "MediaSelect" => 181,
        "LaunchApp1" => 182,
        "LaunchApp2" => 183,
        "Semicolon" => 186,
        "Equal" => 187,
        "Comma" => 188,
        "Minus" => 189,
        "Period" => 190,
        "Slash" => 191,
        "Backquote" => 192,
        "BracketLeft" => 219,
        "Backslash" | "IntlYen" => 220,
        "BracketRight" => 221,
        "Quote" => 222,
        "IntlBackslash" | "IntlRo" => 226,
        "MediaPlay" => 250,
        "MediaPause" => 19,
        _ => return None,
    })
}
impl FromStr for HotKey {
    type Err = String;
    fn from_str(text: &str) -> Result<Self> {
        // Keep the former parser only for saved-setting spelling/alias compatibility.
        // Registration and event delivery never use global-hotkey or RegisterHotKey.
        let canonical = if let Ok(legacy) = text.parse::<global_hotkey::hotkey::HotKey>() {
            use global_hotkey::hotkey::Modifiers;
            let mut parts = Vec::new();
            for (flag, name) in [
                (Modifiers::CONTROL, "Ctrl"),
                (Modifiers::ALT, "Alt"),
                (Modifiers::SHIFT, "Shift"),
                (Modifiers::META, "Super"),
            ] {
                if legacy.mods.contains(flag) {
                    parts.push(name.to_string());
                }
            }
            parts.push(legacy.key.to_string());
            parts.join("+")
        } else {
            text.to_string()
        };
        let text = canonical.as_str();
        let mut chord = Chord {
            modifiers: [0; 4],
            keys: [0; 4],
        };
        for part in text.split('+') {
            let (name, side) = if let Some(s) = part.strip_prefix("Left") {
                (s, 1)
            } else if let Some(s) = part.strip_prefix("Right") {
                (s, 2)
            } else {
                (part, 4)
            };
            let group = match name {
                "Ctrl" => Some(0),
                "Alt" => Some(1),
                "Shift" => Some(2),
                "Super" => Some(3),
                _ => None,
            };
            if let Some(i) = group {
                if chord.modifiers[i] & side != 0
                    || (chord.modifiers[i] != 0 && (side == 4 || chord.modifiers[i] == 4))
                {
                    return Err("Duplicate modifier".into());
                }
                chord.modifiers[i] |= side;
            } else {
                if side != 4 {
                    return Err("Invalid shortcut".into());
                }
                let key = key_name_to_vk(part).ok_or("Unsupported key")? as usize;
                let bit = 1u64 << (key % 64);
                if chord.keys[key / 64] & bit != 0 {
                    return Err("Duplicate key".into());
                }
                chord.keys[key / 64] |= bit;
            }
        }
        let count = chord
            .modifiers
            .iter()
            .map(|v| {
                if *v == 3 {
                    2
                } else if *v != 0 {
                    1
                } else {
                    0
                }
            })
            .sum::<u8>();
        if chord.keys == [0; 4] && count < 2 {
            return Err("修飾キーのみの場合は2つ以上を組み合わせてください".into());
        }
        Ok(Self(chord))
    }
}
impl Chord {
    fn matches(&self, down: &[bool; 256]) -> bool {
        for (i, (l, r)) in [(0xa2, 0xa3), (0xa4, 0xa5), (0xa0, 0xa1), (0x5b, 0x5c)]
            .into_iter()
            .enumerate()
        {
            let actual = (down[l] as u8) | ((down[r] as u8) << 1);
            if if self.modifiers[i] == 4 {
                actual == 0
            } else {
                actual != self.modifiers[i]
            } {
                return false;
            }
        }
        if self.keys != [0; 4] {
            return (0..256)
                .all(|key| self.keys[key / 64] & (1u64 << (key % 64)) == 0 || down[key]);
        }
        // A modifier-only chord is not triggered by releasing a typed base key.
        !(8..=226).any(|k| down[k] && !matches!(k,0x10..=0x12|0x5b..=0x5c|0xa0..=0xa5))
    }
}

// Focus changes and native menu loops can lose a release notification. Refresh
// held keys and modifiers before applying the current low-level event (whose
// asynchronous state Windows has not updated yet).
fn reconcile_keys(down: &mut [bool], pressed: impl Fn(usize) -> bool) {
    for (key, held) in down.iter_mut().enumerate().skip(1) {
        if *held || matches!(key, 0x5b..=0x5c | 0xa0..=0xa5) {
            *held = pressed(key);
        }
    }
}

fn rising_edge(matched: &mut bool, now: bool) -> bool {
    let fire = now && !*matched;
    *matched = now;
    fire
}

pub struct GlobalHotKeyManager {
    #[cfg(feature = "windows-integration")]
    hook: RefCell<Option<Hook>>,
}
impl GlobalHotKeyManager {
    pub fn begin_capture(&self) -> Result<()> {
        #[cfg(feature = "windows-integration")]
        {
            if self.hook.borrow().is_none() {
                *self.hook.borrow_mut() = Some(Hook::new()?);
            }
            HOOK_STATE.with(|s| {
                let mut s = s.lock().unwrap();
                s.capture = true;
                s.recorded = None;
            });
        }
        Ok(())
    }
    pub fn end_capture(&self) {
        #[cfg(feature = "windows-integration")]
        {
            let empty = HOOK_STATE.with(|s| {
                let mut s = s.lock().unwrap();
                s.capture = false;
                s.recorded = None;
                s.keys.is_empty()
            });
            if empty {
                self.hook.borrow_mut().take();
            }
        }
    }
    pub fn new() -> Result<Self> {
        Ok(Self {
            #[cfg(feature = "windows-integration")]
            hook: RefCell::new(None),
        })
    }
    pub fn register(&self, key: HotKey) -> Result<()> {
        #[cfg(feature = "windows-integration")]
        {
            if self.hook.borrow().is_none() {
                *self.hook.borrow_mut() = Some(Hook::new()?);
            }
            HOOK_STATE.with(|s| {
                let mut state = s.lock().unwrap();
                if state.keys.iter().any(|(existing, _)| *existing == key) {
                    return Err("Shortcut already registered in this manager".into());
                }
                state.keys.push((key, false));
                Ok(())
            })
        }
        #[cfg(not(feature = "windows-integration"))]
        {
            let _ = key;
            Err("ショートカット検出にはWindows連携が必要です".into())
        }
    }
    pub fn unregister(&self, key: HotKey) -> Result<()> {
        #[cfg(feature = "windows-integration")]
        {
            let empty = HOOK_STATE.with(|s| {
                let mut state = s.lock().unwrap();
                state.keys.retain(|(existing, _)| *existing != key);
                state.keys.is_empty() && !state.capture
            });
            if empty {
                self.hook.borrow_mut().take();
            }
        }
        #[cfg(not(feature = "windows-integration"))]
        let _ = key;
        Ok(())
    }
}
#[cfg(feature = "windows-integration")]
struct Hook {
    thread_id: u32,
    worker: Option<std::thread::JoinHandle<()>>,
}
#[cfg(feature = "windows-integration")]
#[derive(Default)]
struct HookState {
    keys: Vec<(HotKey, bool)>,
    down: Vec<bool>,
    capture: bool,
    recorded: Option<Vec<u16>>,
    recent: std::collections::VecDeque<InputStamp>,
}
#[cfg(feature = "windows-integration")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InputSource {
    Hook,
    Window,
}
#[cfg(feature = "windows-integration")]
struct InputStamp {
    vk: usize,
    down: bool,
    time: u32,
    source: InputSource,
}
#[cfg(feature = "windows-integration")]
static HOOK_STATE: HookStorage = HookStorage;
#[cfg(feature = "windows-integration")]
struct HookStorage;
#[cfg(feature = "windows-integration")]
impl HookStorage {
    fn with<R>(&self, f: impl FnOnce(&std::sync::Mutex<HookState>) -> R) -> R {
        static STATE: std::sync::OnceLock<std::sync::Mutex<HookState>> = std::sync::OnceLock::new();
        f(STATE.get_or_init(|| {
            std::sync::Mutex::new(HookState {
                keys: Vec::new(),
                down: vec![false; 256],
                capture: false,
                recorded: None,
                recent: Default::default(),
            })
        }))
    }
}
#[cfg(feature = "windows-integration")]
pub(crate) fn take_capture_keys() -> Option<Vec<u16>> {
    HOOK_STATE.with(|s| s.lock().unwrap().recorded.take())
}

#[cfg(feature = "windows-integration")]
impl HookState {
    fn input_event(
        &mut self,
        vk: usize,
        down: bool,
        time: u32,
        source: InputSource,
        pressed: impl Fn(usize) -> bool,
    ) -> Vec<HotKey> {
        if vk >= self.down.len() {
            return Vec::new();
        }
        // Both Windows messages carry the original input timestamp. Remember a
        // bounded history so even a delayed complete press cannot be replayed.
        if let Some(index) = self.recent.iter().position(|event| {
            event.vk == vk && event.down == down && event.time == time && event.source != source
        }) {
            self.recent.remove(index);
            return Vec::new();
        }
        if self.recent.len() == 64 {
            self.recent.pop_front();
        }
        self.recent.push_back(InputStamp {
            vk,
            down,
            time,
            source,
        });
        let was_down = self.down[vk];
        // Window callbacks run after Windows updates the async state. Never
        // mistake this first keydown for a repeat just because it is already down.
        self.reconcile(|key| {
            if key == vk {
                was_down && pressed(key)
            } else {
                pressed(key)
            }
        });
        self.key_event(vk, down)
    }
    fn reconcile(&mut self, pressed: impl Fn(usize) -> bool) {
        reconcile_keys(&mut self.down, pressed);
        let mut down = [false; 256];
        down.copy_from_slice(&self.down);
        for (HotKey(chord), matched) in &mut self.keys {
            if !chord.matches(&down) {
                *matched = false;
            }
        }
    }
    fn key_event(&mut self, vk: usize, down: bool) -> Vec<HotKey> {
        if vk >= 256 {
            return Vec::new();
        }
        let repeated = self.down[vk] == down;
        self.down[vk] = down;
        let mut pressed = [false; 256];
        pressed.copy_from_slice(&self.down);
        if self.capture && down && !repeated {
            let keys: Vec<u16> = (8u16..=226).filter(|k| pressed[*k as usize]).collect();
            let modifiers = keys
                .iter()
                .filter(|k| matches!(**k, 0x5b..=0x5c | 0xa0..=0xa5))
                .count();
            if modifiers >= 2
                || keys
                    .iter()
                    .any(|k| !matches!(*k, 0x10..=0x12 | 0x5b..=0x5c | 0xa0..=0xa5))
            {
                self.recorded = Some(keys);
            }
        }
        let mut fired = Vec::new();
        for (key, matched) in &mut self.keys {
            let HotKey(chord) = key;
            if rising_edge(matched, chord.matches(&pressed)) && down && !repeated && !self.capture {
                fired.push(*key);
            }
        }
        fired
    }
}

#[cfg(feature = "windows-integration")]
pub(crate) fn foreground_key_event(name: &str, down: bool) {
    let vk = match name {
        "ControlLeft" => Some(0xa2),
        "ControlRight" => Some(0xa3),
        "AltLeft" => Some(0xa4),
        "AltRight" => Some(0xa5),
        "ShiftLeft" => Some(0xa0),
        "ShiftRight" => Some(0xa1),
        "SuperLeft" => Some(0x5b),
        "SuperRight" => Some(0x5c),
        _ => key_name_to_vk(name),
    };
    if let Some(vk) = vk {
        let time = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetMessageTime() } as u32;
        process_key_event(vk as usize, down, time, InputSource::Window);
    }
}

#[cfg(feature = "windows-integration")]
fn process_key_event(vk: usize, down: bool, time: u32, source: InputSource) {
    let fired = HOOK_STATE.with(|state| {
        let mut state = state.lock().unwrap();
        state.input_event(vk, down, time, source, |key| unsafe {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key as i32) < 0
        })
    });
    for key in fired {
        if let Some(route) = crate::HOTKEY_ROUTE.get() {
            if let Some((id, sender)) = route.lock().unwrap().as_ref() {
                if *id == key {
                    let _ = sender.send(crate::Command::Toggle);
                }
            }
        }
    }
}

#[cfg(feature = "windows-integration")]
impl Hook {
    fn new() -> Result<Self> {
        use windows_sys::Win32::UI::WindowsAndMessaging::*;
        let (tx, rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name("shortcut-input".into())
            .spawn(move || unsafe {
                let mut msg: MSG = std::mem::zeroed();
                PeekMessageW(&mut msg, std::ptr::null_mut(), 0, 0, PM_NOREMOVE);
                let hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook),
                    windows_sys::Win32::System::LibraryLoader::GetModuleHandleW(std::ptr::null()),
                    0,
                );
                if hook.is_null() {
                    let _ = tx.send(Err(std::io::Error::last_os_error().to_string()));
                    return;
                }
                let id = windows_sys::Win32::System::Threading::GetCurrentThreadId();
                let _ = tx.send(Ok(id));
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
                UnhookWindowsHookEx(hook);
            })
            .map_err(|e| e.to_string())?;
        match rx.recv().map_err(|e| e.to_string())? {
            Ok(thread_id) => Ok(Self {
                thread_id,
                worker: Some(worker),
            }),
            Err(error) => {
                let _ = worker.join();
                Err(error)
            }
        }
    }
}
#[cfg(feature = "windows-integration")]
impl Drop for Hook {
    fn drop(&mut self) {
        unsafe {
            windows_sys::Win32::UI::WindowsAndMessaging::PostThreadMessageW(
                self.thread_id,
                windows_sys::Win32::UI::WindowsAndMessaging::WM_QUIT,
                0,
                0,
            );
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        HOOK_STATE.with(|s| {
            let mut s = s.lock().unwrap();
            s.down.fill(false);
            s.keys.clear();
            s.recorded = None;
            s.recent.clear();
            s.capture = false;
        });
    }
}
#[cfg(feature = "windows-integration")]
unsafe extern "system" fn keyboard_hook(code: i32, wparam: usize, lparam: isize) -> isize {
    use windows_sys::Win32::UI::WindowsAndMessaging::*;
    if code == HC_ACTION as i32 {
        let event = &*(lparam as *const KBDLLHOOKSTRUCT);
        let down = matches!(wparam as u32, WM_KEYDOWN | WM_SYSKEYDOWN);
        let up = matches!(wparam as u32, WM_KEYUP | WM_SYSKEYUP);
        if down || up {
            let extended = event.flags & LLKHF_EXTENDED != 0;
            let vk = match event.vkCode {
                0x03 => 0x13, // Ctrl+Pause is reported as VK_CANCEL.
                0x11 => {
                    if extended {
                        0xa3
                    } else {
                        0xa2
                    }
                }
                0x12 => {
                    if extended {
                        0xa5
                    } else {
                        0xa4
                    }
                }
                0x10 => {
                    if event.scanCode == 0x36 {
                        0xa1
                    } else {
                        0xa0
                    }
                }
                v => v,
            } as usize;
            process_key_event(vk, down, event.time, InputSource::Hook);
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "windows-integration")]
    #[test]
    fn foreground_modifier_chord_works_with_already_updated_async_state() {
        let key: HotKey = "LeftCtrl+RightCtrl".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: false,
            recorded: None,
            recent: Default::default(),
        };
        assert!(state
            .input_event(0xa2, true, 1, InputSource::Window, |vk| vk == 0xa2)
            .is_empty());
        assert_eq!(
            state.input_event(0xa3, true, 2, InputSource::Window, |vk| vk == 0xa2
                || vk == 0xa3),
            vec![key]
        );
        assert!(state
            .input_event(0xa3, true, 3, InputSource::Window, |vk| vk == 0xa2
                || vk == 0xa3)
            .is_empty());
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn delayed_complete_press_from_the_other_route_is_not_replayed() {
        for key in ["LeftCtrl+RightCtrl", "Ctrl+Alt+KeyM", "KeyK+KeyL"] {
            for first in [InputSource::Hook, InputSource::Window] {
                let other = if first == InputSource::Hook {
                    InputSource::Window
                } else {
                    InputSource::Hook
                };
                let shortcut: HotKey = key.parse().unwrap();
                let mut state = HookState {
                    keys: vec![(shortcut, false)],
                    down: vec![false; 256],
                    capture: false,
                    recorded: None,
                    recent: Default::default(),
                };
                let keys: &[usize] = match key {
                    "LeftCtrl+RightCtrl" => &[0xa2, 0xa3],
                    "Ctrl+Alt+KeyM" => &[0xa2, 0xa4, 0x4d],
                    _ => &[0x4b, 0x4c],
                };
                let events: Vec<_> = keys
                    .iter()
                    .map(|vk| (*vk, true))
                    .chain(keys.iter().rev().map(|vk| (*vk, false)))
                    .collect();
                let mut physical = [false; 256];
                let mut fires = Vec::new();
                for (index, (vk, down)) in events.iter().copied().enumerate() {
                    physical[vk] = down;
                    fires.extend(
                        state.input_event(vk, down, index as u32, first, |key| physical[key]),
                    );
                }
                assert_eq!(fires, vec![shortcut], "{key}, {first:?}");
                for (index, (vk, down)) in events.iter().copied().enumerate() {
                    assert!(
                        state
                            .input_event(vk, down, index as u32, other, |_| false)
                            .is_empty(),
                        "{key}, {first:?}"
                    );
                }
                assert!(!state.down.iter().any(|down| *down));
                state.capture = true;
                for (index, (vk, down)) in events.iter().copied().enumerate() {
                    physical[vk] = down;
                    assert!(state
                        .input_event(vk, down, 100 + index as u32, first, |key| physical[key])
                        .is_empty());
                }
                state.capture = false;
                let mut resumed = Vec::new();
                for (index, (vk, down)) in events.iter().copied().enumerate() {
                    physical[vk] = down;
                    resumed.extend(
                        state.input_event(vk, down, 200 + index as u32, first, |key| physical[key]),
                    );
                }
                assert_eq!(resumed, vec![shortcut]);
                for time in 1000..1100 {
                    state.input_event(0x41, false, time, first, |_| false);
                }
                assert_eq!(state.recent.len(), 64);
            }
        }
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn fresh_default_shortcut_works_on_either_side_without_foreground_input() {
        let key: HotKey = crate::Config::default().shortcut.parse().unwrap();
        assert_eq!(key, "control+option+m".parse().unwrap());
        for ctrl in [0xa2, 0xa3] {
            for alt in [0xa4, 0xa5] {
                let mut state = HookState {
                    keys: vec![(key, false)],
                    down: vec![false; 256],
                    capture: false,
                    recorded: None,
                    recent: Default::default(),
                };
                assert!(state.key_event(ctrl, true).is_empty());
                assert!(state.key_event(alt, true).is_empty());
                assert_eq!(state.key_event(0x4d, true), vec![key]);
                assert!(state.key_event(0x4d, true).is_empty());
                state.key_event(0x4d, false);
                state.capture = true;
                assert!(state.key_event(0x4d, true).is_empty());
                state.key_event(0x4d, false);
                state.capture = false;
                assert_eq!(state.key_event(0x4d, true), vec![key]);
                state.key_event(0x4d, false);
                state.key_event(ctrl, false);
                assert!(state.key_event(0x4d, true).is_empty());
            }
        }
    }

    #[test]
    fn legacy_spelling_and_special_keys_keep_their_meaning() {
        for (old, canonical) in [
            ("ctrl+alt+m", "Ctrl+Alt+KeyM"),
            ("CONTROL+OPTION+F10", "Ctrl+Alt+F10"),
            ("cmd+esc", "Super+Escape"),
            ("Ctrl+VolumeUp", "Ctrl+AudioVolumeUp"),
        ] {
            assert_eq!(old.parse::<HotKey>().unwrap(), canonical.parse().unwrap());
        }
        for key in [
            "Ctrl+Pause",
            "Ctrl+IntlYen",
            "Alt+NumpadEnter",
            "MediaTrackNext",
        ] {
            assert!(key.parse::<HotKey>().is_ok(), "{key}");
        }
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn reconciliation_rearms_a_chord_after_a_lost_release() {
        let key: HotKey = "Ctrl+Alt+KeyM".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: false,
            recorded: None,
            recent: Default::default(),
        };
        state.key_event(0xa2, true);
        state.key_event(0xa4, true);
        assert_eq!(state.key_event(0x4d, true), vec![key]);
        // Windows still reports the state before the new keydown: M was released.
        state.reconcile(|key| key == 0xa2 || key == 0xa4);
        assert_eq!(state.key_event(0x4d, true), vec![key]);
        // A genuine repeat while M remains held must not fire again.
        state.reconcile(|key| key == 0xa2 || key == 0xa4 || key == 0x4d);
        assert!(state.key_event(0x4d, true).is_empty());
    }
    #[test]
    fn plain_key_chords_parse_in_either_order_and_require_all_keys() {
        let first: HotKey = "KeyK+KeyL".parse().unwrap();
        let second: HotKey = "L+K".parse().unwrap();
        assert_eq!(first, second);

        let HotKey(chord) = first;
        let mut down = [false; 256];
        down[0x4b] = true;
        assert!(!chord.matches(&down));
        down[0x4c] = true;
        assert!(chord.matches(&down));
        down[0xa2] = true;
        assert!(!chord.matches(&down));
        assert!("KeyK+KeyK".parse::<HotKey>().is_err());
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn plain_chords_fire_once_per_complete_hook_press() {
        let key: HotKey = "KeyK+KeyL".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: false,
            recorded: None,
            recent: Default::default(),
        };
        for _ in 0..2 {
            assert!(state.key_event(0x4b, true).is_empty());
            assert_eq!(state.key_event(0x4c, true), vec![key]);
            assert!(state.key_event(0x4c, true).is_empty());
            state.key_event(0x4b, false);
            assert_eq!(state.key_event(0x4b, true), vec![key]);
            state.key_event(0x4b, false);
            state.key_event(0x4c, false);
        }
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn sided_hook_chords_stop_during_capture_and_resume_after_cancel() {
        let key: HotKey = "RightCtrl+LeftAlt".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: true,
            recorded: None,
            recent: Default::default(),
        };
        assert!(state.key_event(0xa3, true).is_empty());
        assert!(state.key_event(0xa4, true).is_empty());
        assert_eq!(state.recorded, Some(vec![0xa3, 0xa4]));
        state.key_event(0xa3, false);
        state.key_event(0xa4, false);
        state.capture = false;
        assert!(state.key_event(0xa3, true).is_empty());
        assert_eq!(state.key_event(0xa4, true), vec![key]);
        assert!(state.key_event(0xa4, true).is_empty());
    }
    #[test]
    fn a_held_chord_fires_only_on_the_press_edge() {
        let mut matched = false;
        assert!(rising_edge(&mut matched, true));
        assert!(!rising_edge(&mut matched, true));
        assert!(!rising_edge(&mut matched, false));
        assert!(rising_edge(&mut matched, true));
    }
    #[test]
    fn lost_release_cannot_block_a_modifier_chord_after_focus_change() {
        let mut down = [false; 256];
        down[0x4d] = true;
        down[0xa2] = true;
        reconcile_keys(&mut down, |key| key == 0xa3);
        down[0xa5] = true;
        let HotKey(chord) = "RightCtrl+RightAlt".parse().unwrap();
        assert!(chord.matches(&down));
        assert!(!down[0x4d] && !down[0xa2]);
    }
    #[test]
    fn side_specific_and_modifier_only_match() {
        let HotKey(c) = "RightCtrl+RightAlt".parse().unwrap();
        let mut down = [false; 256];
        down[0xa3] = true;
        down[0xa5] = true;
        assert!(c.matches(&down));
        down[0xa3] = false;
        down[0xa2] = true;
        assert!(!c.matches(&down));
        down[0xa2] = false;
        down[0xa3] = true;
        down[0x4d] = true;
        assert!(!c.matches(&down));
        let HotKey(c) = "RightCtrl+LeftAlt+KeyM".parse().unwrap();
        down[0xa5] = false;
        down[0xa4] = true;
        assert!(c.matches(&down));
        assert!("RightCtrl".parse::<HotKey>().is_err());
        assert!("RightCtrl+RightCtrl".parse::<HotKey>().is_err());
        assert!("Ctrl+Alt+KeyM".parse::<HotKey>().is_ok());
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn native_hook_can_register_suspend_and_resume() {
        let manager = GlobalHotKeyManager::new().unwrap();
        let key = "RightCtrl+RightAlt".parse().unwrap();
        manager.register(key).unwrap();
        manager.begin_capture().unwrap();
        manager.unregister(key).unwrap();
        manager.end_capture();
        assert!(manager.hook.borrow().is_none());
        manager.register(key).unwrap();
        manager.unregister(key).unwrap();
        assert!(manager.hook.borrow().is_none());
    }
}
