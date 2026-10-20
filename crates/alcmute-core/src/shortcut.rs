use crate::Result;
use std::{cell::RefCell, str::FromStr};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HotKey {
    Standard(global_hotkey::hotkey::HotKey),
    Sided(Chord),
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Chord {
    modifiers: [u8; 4],
    keys: [u64; 4],
}
impl HotKey {
    pub fn id(self) -> u32 {
        match self {
            Self::Standard(key) => key.id(),
            Self::Sided(c) => {
                let mut hash = 2166136261u32;
                for byte in c
                    .modifiers
                    .into_iter()
                    .chain(c.keys.into_iter().flat_map(u64::to_le_bytes))
                {
                    hash = (hash ^ byte as u32).wrapping_mul(16777619);
                }
                hash | 0x80000000
            }
        }
    }
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
        "Pause" => 19,
        "CapsLock" => 20,
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
        "ContextMenu" => 93,
        "NumpadMultiply" => 106,
        "NumpadAdd" => 107,
        "NumpadComma" => 108,
        "NumpadSubtract" => 109,
        "NumpadDecimal" => 110,
        "NumpadDivide" => 111,
        "NumLock" => 144,
        "ScrollLock" => 145,
        "Semicolon" => 186,
        "Equal" => 187,
        "Comma" => 188,
        "Minus" => 189,
        "Period" => 190,
        "Slash" => 191,
        "Backquote" => 192,
        "BracketLeft" => 219,
        "Backslash" => 220,
        "BracketRight" => 221,
        "Quote" => 222,
        "IntlBackslash" => 226,
        _ => return None,
    })
}
impl FromStr for HotKey {
    type Err = String;
    fn from_str(text: &str) -> Result<Self> {
        if !text
            .split('+')
            .any(|p| p.starts_with("Left") || p.starts_with("Right"))
        {
            if let Ok(key) = text.parse() {
                return Ok(Self::Standard(key));
            }
        }
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
        Ok(Self::Sided(chord))
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
    for key in 8..=226 {
        if down[key] || matches!(key, 0x5b..=0x5c | 0xa0..=0xa5) {
            down[key] = pressed(key);
        }
    }
}

fn rising_edge(matched: &mut bool, now: bool) -> bool {
    let fire = now && !*matched;
    *matched = now;
    fire
}

pub struct GlobalHotKeyManager {
    standard: global_hotkey::GlobalHotKeyManager,
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
            standard: global_hotkey::GlobalHotKeyManager::new().map_err(|e| e.to_string())?,
            #[cfg(feature = "windows-integration")]
            hook: RefCell::new(None),
        })
    }
    pub fn register(&self, key: HotKey) -> Result<()> {
        match key {
            HotKey::Standard(k) => self.standard.register(k).map_err(|e| e.to_string()),
            HotKey::Sided(_) => {
                #[cfg(feature = "windows-integration")]
                {
                    if self.hook.borrow().is_none() {
                        *self.hook.borrow_mut() = Some(Hook::new()?);
                    }
                    HOOK_STATE.with(|s| s.lock().unwrap().keys.push((key, false)));
                    Ok(())
                }
                #[cfg(not(feature = "windows-integration"))]
                {
                    Err("左右別ショートカットはSlint版で利用できます".into())
                }
            }
        }
    }
    pub fn unregister(&self, key: HotKey) -> Result<()> {
        match key {
            HotKey::Standard(k) => self.standard.unregister(k).map_err(|e| e.to_string()),
            HotKey::Sided(_) => {
                #[cfg(feature = "windows-integration")]
                {
                    let empty = HOOK_STATE.with(|s| {
                        let mut s = s.lock().unwrap();
                        s.keys.retain(|(k, _)| *k != key);
                        s.keys.is_empty() && !s.capture
                    });
                    if empty {
                        self.hook.borrow_mut().take();
                    }
                }
                Ok(())
            }
        }
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
            })
        }))
    }
}
#[cfg(feature = "windows-integration")]
pub(crate) fn take_capture_keys() -> Option<Vec<u16>> {
    HOOK_STATE.with(|s| s.lock().unwrap().recorded.take())
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
        "Escape" => Some(0x1b),
        _ => key_name_to_vk(name),
    };
    if let Some(vk) = vk {
        process_key_event(vk as usize, down, true);
    }
}

#[cfg(feature = "windows-integration")]
impl HookState {
    fn input_event(
        &mut self,
        vk: usize,
        down: bool,
        from_settings: bool,
        foreground_ours: bool,
    ) -> Vec<HotKey> {
        // One authoritative stream per foreground state prevents delayed global
        // duplicates from replaying a whole press after winit saw its release.
        if from_settings != foreground_ours {
            return Vec::new();
        }
        self.key_event(vk, down)
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
            let HotKey::Sided(chord) = key else {
                continue;
            };
            if rising_edge(matched, chord.matches(&pressed)) && down && !repeated && !self.capture {
                fired.push(*key);
            }
        }
        fired
    }
}

#[cfg(feature = "windows-integration")]
fn process_key_event(vk: usize, down: bool, from_settings: bool) {
    let foreground_ours = crate::windows::foreground_is_ours();
    let fired = HOOK_STATE.with(|state| {
        state
            .lock()
            .unwrap()
            .input_event(vk, down, from_settings, foreground_ours)
    });
    for key in fired {
        if let Some(route) = crate::HOTKEY_ROUTE.get() {
            if let Some((id, sender)) = route.lock().unwrap().as_ref() {
                if *id == key.id() {
                    let _ = sender.send(crate::Command::Toggle);
                }
            }
        }
    }
}

#[cfg(feature = "windows-integration")]
pub(crate) fn foreground_focus_gained() {
    HOOK_STATE.with(|storage| {
        let mut state = storage.lock().unwrap();
        reconcile_keys(&mut state.down, |key| unsafe {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key as i32) < 0
        });
        let mut pressed = [false; 256];
        pressed.copy_from_slice(&state.down);
        for (key, matched) in &mut state.keys {
            if let HotKey::Sided(chord) = key {
                *matched = chord.matches(&pressed);
            }
        }
    });
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
            process_key_event(vk, down, false);
        }
    }
    CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plain_key_chords_parse_in_either_order_and_require_all_keys() {
        let first: HotKey = "KeyK+KeyL".parse().unwrap();
        let second: HotKey = "L+K".parse().unwrap();
        assert_eq!(first, second);
        assert_eq!(first.id(), second.id());
        let HotKey::Sided(chord) = first else {
            panic!()
        };
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
    fn plain_chords_fire_once_per_complete_press_in_both_input_routes() {
        let key: HotKey = "KeyK+KeyL".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: false,
            recorded: None,
        };
        for foreground in [true, false] {
            assert!(state
                .input_event(0x4b, true, foreground, foreground)
                .is_empty());
            assert_eq!(
                state.input_event(0x4c, true, foreground, foreground),
                vec![key]
            );
            assert!(state
                .input_event(0x4c, true, foreground, foreground)
                .is_empty());
            state.input_event(0x4b, false, foreground, foreground);
            assert_eq!(
                state.input_event(0x4b, true, foreground, foreground),
                vec![key]
            );
            state.input_event(0x4b, false, foreground, foreground);
            state.input_event(0x4c, false, foreground, foreground);
        }
    }
    #[cfg(feature = "windows-integration")]
    #[test]
    fn window_keys_work_without_replaying_global_duplicates_or_firing_during_capture() {
        let key: HotKey = "RightCtrl+LeftAlt".parse().unwrap();
        let mut state = HookState {
            keys: vec![(key, false)],
            down: vec![false; 256],
            capture: false,
            recorded: None,
        };
        assert!(state.input_event(0xa3, true, false, true).is_empty());
        assert!(state.input_event(0xa4, true, false, true).is_empty());
        assert!(state.input_event(0xa3, true, true, true).is_empty());
        assert_eq!(state.input_event(0xa4, true, true, true), vec![key]);
        assert!(state.input_event(0xa4, true, true, true).is_empty());
        state.input_event(0xa4, false, true, true);
        state.input_event(0xa3, false, true, true);
        assert!(state.input_event(0xa3, true, false, true).is_empty());
        assert!(state.input_event(0xa4, true, false, true).is_empty());
        state.capture = true;
        state.input_event(0xa3, true, true, true);
        assert!(state.input_event(0xa4, true, true, true).is_empty());
        assert_eq!(state.recorded, Some(vec![0xa3, 0xa4]));
        state.input_event(0xa4, false, true, true);
        state.input_event(0xa3, false, true, true);
        state.capture = false;
        state.input_event(0xa3, true, false, false);
        assert_eq!(state.input_event(0xa4, true, false, false), vec![key]);
    }
    #[test]
    fn foreground_and_global_routes_share_one_press_edge() {
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
        let HotKey::Sided(chord) = "RightCtrl+RightAlt".parse().unwrap() else {
            panic!()
        };
        assert!(chord.matches(&down));
        assert!(!down[0x4d] && !down[0xa2]);
    }
    #[test]
    fn side_specific_and_modifier_only_match() {
        let HotKey::Sided(c) = "RightCtrl+RightAlt".parse().unwrap() else {
            panic!()
        };
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
        let HotKey::Sided(c) = "RightCtrl+LeftAlt+KeyM".parse().unwrap() else {
            panic!()
        };
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
