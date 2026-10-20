//! Windows-only integration used by the selected Slint frontend.
use crate::{Config, Connection, Event, Notify, Options, Result, Status};
use serde_json::Value;
use std::{
    net::SocketAddr,
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use windows_sys::Win32::{
    Foundation::{
        CloseHandle, ERROR_INSUFFICIENT_BUFFER, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
    },
    NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_LISTENER,
    },
    Networking::WinSock::AF_INET,
    System::{
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
            TH32CS_SNAPPROCESS,
        },
        Threading::{OpenProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE},
    },
};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ},
    RegKey,
};

struct Handle(HANDLE);
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
fn vrchat_pids() -> Result<Vec<u32>> {
    // The snapshot and PROCESSENTRY32W are owned and sized for this API.
    unsafe {
        let raw = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if raw == INVALID_HANDLE_VALUE {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let snapshot = Handle(raw);
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut pids = Vec::new();
        let mut available = Process32FirstW(snapshot.0, &mut entry);
        while available != 0 {
            let length = entry
                .szExeFile
                .iter()
                .position(|c| *c == 0)
                .unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length])
                .eq_ignore_ascii_case("VRChat.exe")
            {
                pids.push(entry.th32ProcessID);
            }
            available = Process32NextW(snapshot.0, &mut entry);
        }
        Ok(pids)
    }
}
fn tcp_ports(pids: &[u32]) -> Result<Vec<(u32, u16)>> {
    let mut size = 0u32;
    unsafe {
        GetExtendedTcpTable(
            std::ptr::null_mut(),
            &mut size,
            0,
            AF_INET as u32,
            TCP_TABLE_OWNER_PID_LISTENER,
            0,
        );
    }
    for _ in 0..3 {
        // u32 storage has the alignment required by the Windows table.
        let mut storage = vec![0u32; (size as usize).div_ceil(4).max(1)];
        let result = unsafe {
            GetExtendedTcpTable(
                storage.as_mut_ptr().cast(),
                &mut size,
                0,
                AF_INET as u32,
                TCP_TABLE_OWNER_PID_LISTENER,
                0,
            )
        };
        if result == ERROR_INSUFFICIENT_BUFFER {
            continue;
        }
        if result != 0 {
            return Err(format!("Cannot enumerate VRChat TCP endpoints: {result}"));
        }
        let count = storage[0] as usize;
        let row_size = std::mem::size_of::<MIB_TCPROW_OWNER_PID>();
        let mut ports = Vec::new();
        for index in 0..count {
            let offset = 4 + index * row_size;
            if offset + row_size > size as usize {
                return Err("Truncated TCP table".into());
            }
            let row = unsafe {
                std::ptr::read_unaligned(
                    storage
                        .as_ptr()
                        .cast::<u8>()
                        .add(offset)
                        .cast::<MIB_TCPROW_OWNER_PID>(),
                )
            };
            let ip = std::net::Ipv4Addr::from(row.dwLocalAddr.to_ne_bytes());
            if pids.contains(&row.dwOwningPid) && (ip.is_loopback() || ip.is_unspecified()) {
                ports.push((row.dwOwningPid, u16::from_be(row.dwLocalPort as u16)));
            }
        }
        return Ok(ports);
    }
    Err("TCP endpoints changed during discovery".into())
}
fn get(agent: &ureq::Agent, port: u16, path: &str) -> Result<Value> {
    agent
        .get(format!("http://127.0.0.1:{port}{path}"))
        .call()
        .map_err(|e| e.to_string())?
        .body_mut()
        .read_json::<Value>()
        .map_err(|e| e.to_string())
}
pub fn query_mute(value: &Value) -> Result<bool> {
    if value.get("FULL_PATH").and_then(Value::as_str) != Some("/avatar/parameters/MuteSelf") {
        return Err("Unexpected OSCQuery node".into());
    }
    let values = value
        .get("VALUE")
        .and_then(Value::as_array)
        .ok_or("Missing OSCQuery VALUE")?;
    if values.len() != 1 {
        return Err("Invalid OSCQuery VALUE length".into());
    }
    values[0]
        .as_bool()
        .ok_or("MuteSelf is not a boolean".into())
}
struct Endpoint {
    port: u16,
    pid: u32,
    process: Option<Handle>,
}
impl Endpoint {
    fn exited(&self) -> bool {
        match &self.process {
            Some(handle) => unsafe { WaitForSingleObject(handle.0, 0) == WAIT_OBJECT_0 },
            None => vrchat_pids().is_ok_and(|pids| !pids.contains(&self.pid)),
        }
    }
}
fn discover(agent: &ureq::Agent, target_port: u16) -> Result<(bool, Option<Endpoint>)> {
    let pids = vrchat_pids()?;
    if pids.is_empty() {
        return Ok((false, None));
    }
    for (pid, port) in tcp_ports(&pids)? {
        if let Ok(info) = get(agent, port, "/?HOST_INFO") {
            if info.get("OSC_PORT").and_then(Value::as_u64) == Some(target_port as u64) {
                let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
                return Ok((
                    true,
                    Some(Endpoint {
                        port,
                        pid,
                        process: if process.is_null() {
                            None
                        } else {
                            Some(Handle(process))
                        },
                    }),
                ));
            }
        }
    }
    Ok((true, None))
}
fn publish(
    status: &Mutex<Status>,
    notify: &Notify,
    connection: Connection,
    muted: Option<bool>,
    sequence: Option<u64>,
) {
    let mut state = status.lock().unwrap();
    // A UDP update received during the HTTP request is newer than its snapshot.
    if sequence.is_some_and(|value| state.sequence != value) {
        return;
    }
    let changed =
        state.connection != connection || state.muted != muted || !state.detail.is_empty();
    if !changed {
        return;
    }
    state.connection = connection;
    state.muted = muted;
    state.detail.clear();
    state.sequence += 1;
    drop(state);
    notify(Event::Status);
}
pub(crate) fn spawn_monitor(
    config: Config,
    status: Arc<Mutex<Status>>,
    notify: Notify,
    stop: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        let agent: ureq::Agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_millis(500)))
            .proxy(None)
            .max_redirects(0)
            .build()
            .into();
        let port = config
            .target
            .parse::<SocketAddr>()
            .expect("validated OSC target")
            .port();
        let mut cached: Option<Endpoint> = None;
        while !stop.load(Ordering::Acquire) {
            if cached.as_ref().is_some_and(Endpoint::exited) {
                cached = None;
                publish(&status, &notify, Connection::Disconnected, None, None);
            }
            if cached.is_none() {
                match discover(&agent, port) {
                    Ok((_, Some(endpoint))) => cached = Some(endpoint),
                    Ok((running, None)) => publish(
                        &status,
                        &notify,
                        if running {
                            Connection::Unavailable
                        } else {
                            Connection::Disconnected
                        },
                        None,
                        None,
                    ),
                    Err(_) => publish(&status, &notify, Connection::Unavailable, None, None),
                }
            }
            if let Some(endpoint) = &cached {
                let sequence = status.lock().unwrap().sequence;
                match get(&agent, endpoint.port, "/avatar/parameters/MuteSelf")
                    .and_then(|json| query_mute(&json))
                {
                    Ok(muted) => publish(
                        &status,
                        &notify,
                        Connection::Connected,
                        Some(muted),
                        Some(sequence),
                    ),
                    Err(_) => {
                        publish(
                            &status,
                            &notify,
                            Connection::Unavailable,
                            None,
                            Some(sequence),
                        );
                        cached = None;
                    }
                }
            }
            if !stop.load(Ordering::Acquire) {
                thread::park_timeout(Duration::from_secs(2));
            }
        }
    })
}

// Test mode uses a non-startup key, never the user's real sign-in registration.
fn startup_path(options: &Options) -> &'static str {
    if options.benchmark {
        r"Software\VRChatMuteController\Testing\Run"
    } else {
        r"Software\Microsoft\Windows\CurrentVersion\Run"
    }
}
const STARTUP_NAME: &str = "VRChatMuteController";
pub fn startup_snapshot(options: &Options) -> Result<Option<String>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = match hkcu.open_subkey_with_flags(startup_path(options), KEY_READ) {
        Ok(key) => key,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Cannot read sign-in startup setting: {error}")),
    };
    match key.get_value(STARTUP_NAME) {
        Ok(command) => Ok(Some(command)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("Cannot read sign-in startup setting: {error}")),
    }
}
pub fn quote_arg(arg: &str) -> String {
    let mut result = String::from("\"");
    let mut slashes = 0;
    for character in arg.chars() {
        if character == '\\' {
            slashes += 1;
            continue;
        }
        if character == '"' {
            result.push_str(&"\\".repeat(slashes * 2 + 1));
        } else {
            result.push_str(&"\\".repeat(slashes));
        }
        slashes = 0;
        result.push(character);
    }
    result.push_str(&"\\".repeat(slashes * 2));
    result.push('"');
    result
}
fn startup_command(exe: &Path, directory: &Path) -> Result<String> {
    let exe = exe
        .to_str()
        .ok_or("Executable path must be valid Unicode")?;
    let directory = directory
        .to_str()
        .ok_or("Settings path must be valid Unicode")?;
    let command = format!("{} --data-dir {}", quote_arg(exe), quote_arg(directory));
    if command.encode_utf16().count() > 260 {
        return Err("Startup command exceeds the Windows Run limit (260 characters). Move the app/settings to a shorter path.".into());
    }
    Ok(command)
}
fn expected_command(options: &Options) -> Result<String> {
    startup_command(
        &std::env::current_exe().map_err(|e| e.to_string())?,
        &std::path::absolute(&options.data_dir).map_err(|e| e.to_string())?,
    )
}
pub fn startup_enabled(options: &Options) -> Result<bool> {
    let Some(saved) = startup_snapshot(options)? else {
        return Ok(false);
    };
    Ok(expected_command(options).is_ok_and(|expected| saved == expected))
}
pub fn restore_startup(options: &Options, command: Option<&str>) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    // Avoid creating the Run key when the user has never enabled startup.
    if command.is_none() && startup_snapshot(options)?.is_none() {
        return Ok(());
    }
    let (key, _) = hkcu
        .create_subkey(startup_path(options))
        .map_err(|e| e.to_string())?;
    if let Some(command) = command {
        key.set_value(STARTUP_NAME, &command)
            .map_err(|e| e.to_string())
    } else {
        match key.delete_value(STARTUP_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.to_string()),
        }
    }
}
pub fn set_startup(options: &Options, enabled: bool) -> Result<()> {
    let command = if enabled {
        Some(expected_command(options)?)
    } else {
        None
    };
    if startup_snapshot(options)? == command {
        return Ok(());
    }
    restore_startup(options, command.as_deref())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn query_requires_a_single_boolean_on_the_right_node() {
        assert_eq!(
            query_mute(
                &serde_json::json!({"FULL_PATH":"/avatar/parameters/MuteSelf","VALUE":[true]})
            ),
            Ok(true)
        );
        assert!(query_mute(
            &serde_json::json!({"FULL_PATH":"/avatar/parameters/MuteSelf","VALUE":[1]})
        )
        .is_err());
        assert!(query_mute(&serde_json::json!({"FULL_PATH":"/other","VALUE":[false]})).is_err());
        assert!(query_mute(
            &serde_json::json!({"FULL_PATH":"/avatar/parameters/MuteSelf","VALUE":[true,false]})
        )
        .is_err());
    }
    #[test]
    fn disconnect_clears_last_mute_and_reconnect_updates_it() {
        let status = Mutex::new(Status {
            muted: Some(false),
            connection: Connection::Connected,
            ..Default::default()
        });
        let notify: Notify = Arc::new(|_| {});
        publish(&status, &notify, Connection::Disconnected, None, None);
        assert_eq!(status.lock().unwrap().muted, None);
        publish(&status, &notify, Connection::Connected, Some(true), None);
        assert_eq!(status.lock().unwrap().muted, Some(true));
        let seq = status.lock().unwrap().sequence;
        publish(
            &status,
            &notify,
            Connection::Connected,
            Some(false),
            Some(seq - 1),
        );
        assert_eq!(
            status.lock().unwrap().muted,
            Some(true),
            "stale HTTP response must not override UDP"
        );
    }
    #[test]
    fn startup_quotes_paths_and_preserves_data_directory() {
        assert_eq!(
            quote_arg("C:\\Apps With Spaces\\mute.exe"),
            "\"C:\\Apps With Spaces\\mute.exe\""
        );
        assert_eq!(quote_arg("C:\\"), "\"C:\\\\\"");
        let command = startup_command(
            Path::new("C:\\Apps\\mute.exe"),
            Path::new("D:\\My Settings"),
        )
        .unwrap();
        assert_eq!(
            command,
            "\"C:\\Apps\\mute.exe\" --data-dir \"D:\\My Settings\""
        );
        assert!(!command.contains("--test-mode"));
    }
    #[test]
    fn registry_round_trip_uses_only_the_non_startup_test_key() {
        let options = Options {
            data_dir: std::env::temp_dir().join(format!("vmc-{}", std::process::id())),
            benchmark: true,
        };
        let previous = startup_snapshot(&options).unwrap();
        set_startup(&options, true).unwrap();
        assert!(startup_enabled(&options).unwrap());
        std::fs::create_dir(&options.data_dir).unwrap();
        let settings = options.data_dir.join("settings.toml");
        let pending = options.data_dir.join("settings.pending.toml");
        std::fs::create_dir(&settings).unwrap();
        std::fs::write(&pending, "").unwrap();
        assert!(crate::commit_settings(&options, &Config::default(), &pending).is_err());
        assert!(
            startup_enabled(&options).unwrap(),
            "Failed reset must restore startup registration"
        );
        std::fs::remove_dir(&settings).unwrap();
        std::fs::write(&settings, "shortcut = 'Ctrl+Alt+KeyN'").unwrap();
        crate::commit_settings(&options, &Config::default(), &pending).unwrap();
        assert!(
            !settings.exists() && !pending.exists(),
            "Reset removes saved preferences"
        );
        assert!(!startup_enabled(&options).unwrap());
        restore_startup(&options, previous.as_deref()).unwrap();
        std::fs::remove_dir(&options.data_dir).unwrap();
    }
}

pub fn queued_keyboard_keys() -> Result<Vec<u16>> {
    let mut keys = [0u8; 256];
    if unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetKeyboardState(keys.as_mut_ptr())
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    Ok((8u16..=254)
        .filter(|key| keys[*key as usize] & 0x80 != 0)
        .collect())
}
pub fn any_keyboard_key_down() -> bool {
    (8..=254).any(|key| unsafe {
        windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(key) < 0
    })
}

/// Physical key state, independent of winit's buffered keyboard message queue.
pub fn pressed_keyboard_keys() -> Vec<u16> {
    // 0xF0..0xFE are Japanese IME mode keys, not held physical keys. Some
    // layouts report these continuously, preventing any release wait finishing.
    (8u16..=0xE2)
        .filter(|key| unsafe {
            windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState(*key as i32) < 0
        })
        .collect()
}

pub fn recorded_keyboard_keys() -> Option<Vec<u16>> {
    crate::shortcut::take_capture_keys()
}

pub fn foreground_key_event(name: &str, down: bool) {
    crate::shortcut::foreground_key_event(name, down);
}
pub fn foreground_focus_gained() {
    crate::shortcut::foreground_focus_gained();
}
pub fn foreground_is_ours() -> bool {
    unsafe {
        let window = windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow();
        let mut pid = 0;
        windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(window, &mut pid);
        pid == std::process::id()
    }
}

pub fn double_click_time() -> u32 {
    unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime() }
}

/// The settings window owns this modal; No/Escape leaves the tool running.
pub fn confirm_tool_exit() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO,
    };
    if !foreground_is_ours() {
        return false;
    }
    let title: Vec<u16> = "ツールを終了\0".encode_utf16().collect();
    let message: Vec<u16> = "AlcMuteを終了しますか？\n\n終了すると、ショートカットやトレイからのミュート操作は利用できなくなります。\n未保存の設定変更は保存されません。\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(
            GetForegroundWindow(),
            message.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

/// A modal, owned confirmation. No/Escape is the default and changes nothing.
pub fn confirm_settings_reset() -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONWARNING, MB_YESNO,
    };
    if !foreground_is_ours() {
        return false;
    }
    let title: Vec<u16> = "すべての設定を初期化\0".encode_utf16().collect();
    let message: Vec<u16> = "保存した設定を削除し、すべての項目を初期値に戻します。\nWindowsサインイン時の自動起動も無効になります。\n\n初期化しますか？\0".encode_utf16().collect();
    unsafe {
        MessageBoxW(
            GetForegroundWindow(),
            message.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_ICONWARNING | MB_DEFBUTTON2,
        ) == IDYES
    }
}
