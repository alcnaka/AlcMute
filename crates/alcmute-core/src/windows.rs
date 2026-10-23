//! Windows-only integration used by the selected Slint frontend.
use crate::{Config, Connection, Event, Notify, Result, Status};
use serde_json::Value;
use std::{
    net::SocketAddr,
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

struct Handle(HANDLE);
#[path = "ipc.rs"]
pub mod ipc;
impl Drop for Handle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

/// Local namespace isolates sessions; the token SID isolates users in a session.
pub struct SingleInstance {
    _handle: Handle,
}

impl SingleInstance {
    pub fn installer_guard() -> Result<Option<Self>> {
        Self::named("Local\\Alcnaka.AlcMute.Installation")
    }
    pub fn acquire() -> Result<Option<Self>> {
        Self::named(&format!("Local\\Alcnaka.AlcMute.{}", current_user_id()?))
    }
    fn named(name: &str) -> Result<Option<Self>> {
        use windows_sys::Win32::{
            Foundation::{GetLastError, ERROR_ALREADY_EXISTS},
            System::Threading::CreateMutexW,
        };
        let name: Vec<u16> = name.encode_utf16().chain(Some(0)).collect();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) };
        if handle.is_null() {
            return Err(std::io::Error::last_os_error().to_string());
        }
        let exists = unsafe { GetLastError() } == ERROR_ALREADY_EXISTS;
        let guard = Self {
            _handle: Handle(handle),
        };
        if exists {
            Ok(None)
        } else {
            Ok(Some(guard))
        }
    }
}

fn current_user_id() -> Result<String> {
    use windows_sys::Win32::{
        Security::{GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
    };
    let mut token = std::ptr::null_mut();
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let token = Handle(token);
    let mut size = 0;
    unsafe {
        GetTokenInformation(token.0, TokenUser, std::ptr::null_mut(), 0, &mut size);
    }
    if size == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    // usize storage supplies the alignment required by TOKEN_USER.
    let mut buffer = vec![0usize; (size as usize).div_ceil(std::mem::size_of::<usize>())];
    if unsafe {
        GetTokenInformation(
            token.0,
            TokenUser,
            buffer.as_mut_ptr().cast(),
            size,
            &mut size,
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let user = unsafe { &*buffer.as_ptr().cast::<TOKEN_USER>() };
    let bytes = unsafe {
        std::slice::from_raw_parts(
            user.User.Sid.cast::<u8>(),
            GetLengthSid(user.User.Sid) as usize,
        )
    };
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

pub fn show_error(title: &str, message: &str) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
/// Called on the existing process's UI thread after activation has been acknowledged.
pub fn show_existing_instance_notice() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, MessageBoxW, MB_ICONINFORMATION, MB_OK,
    };
    let owner = if foreground_is_ours() {
        unsafe { GetForegroundWindow() }
    } else {
        std::ptr::null_mut()
    };
    let title: Vec<u16> = "AlcMuteは起動済みです\0".encode_utf16().collect();
    let message: Vec<u16> =
        "AlcMuteはすでに起動しているため、既存のAlcMuteの設定画面を表示しました。\0"
            .encode_utf16()
            .collect();
    unsafe {
        MessageBoxW(
            owner,
            message.as_ptr(),
            title.as_ptr(),
            MB_OK | MB_ICONINFORMATION,
        );
    }
}

pub fn confirm(title: &str, message: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO,
    };
    let title: Vec<u16> = title.encode_utf16().chain(Some(0)).collect();
    let message: Vec<u16> = message.encode_utf16().chain(Some(0)).collect();
    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            message.as_ptr(),
            title.as_ptr(),
            MB_YESNO | MB_DEFBUTTON2 | MB_ICONQUESTION,
        ) == IDYES
    }
}
pub fn wait_for_update_parent(pid: u32, expected: &std::path::Path) -> Result<()> {
    use windows_sys::Win32::{
        Foundation::{GetLastError, ERROR_INVALID_PARAMETER},
        System::Threading::{QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION},
    };
    let raw = unsafe {
        OpenProcess(
            PROCESS_SYNCHRONIZE | PROCESS_QUERY_LIMITED_INFORMATION,
            0,
            pid,
        )
    };
    if raw.is_null() {
        return if unsafe { GetLastError() } == ERROR_INVALID_PARAMETER {
            Ok(())
        } else {
            Err(std::io::Error::last_os_error().to_string())
        };
    }
    let handle = Handle(raw);
    let mut text = vec![0u16; 32768];
    let mut size = text.len() as u32;
    if unsafe { QueryFullProcessImageNameW(handle.0, 0, text.as_mut_ptr(), &mut size) } == 0 {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let actual = std::path::PathBuf::from(String::from_utf16_lossy(&text[..size as usize]));
    if std::fs::canonicalize(actual).ok() != std::fs::canonicalize(expected).ok() {
        return Err("Update parent executable mismatch".into());
    }
    if unsafe { WaitForSingleObject(handle.0, 30000) } != WAIT_OBJECT_0 {
        return Err("AlcMuteの終了待ちがタイムアウトしました。既存EXEは置換していません。".into());
    }
    Ok(())
}
pub fn create_private_directory(path: &std::path::Path) -> Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::{
        Foundation::LocalFree,
        Security::{
            Authorization::ConvertStringSecurityDescriptorToSecurityDescriptorW,
            SECURITY_ATTRIBUTES,
        },
        Storage::FileSystem::CreateDirectoryW,
    };
    let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    let sddl: Vec<u16> = format!("D:P(A;OICI;GA;;;{})", ipc::sid_string()?)
        .encode_utf16()
        .chain(Some(0))
        .collect();
    let mut descriptor = std::ptr::null_mut();
    if unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl.as_ptr(),
            1,
            &mut descriptor,
            std::ptr::null_mut(),
        )
    } == 0
    {
        return Err(std::io::Error::last_os_error().to_string());
    }
    let attributes = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: descriptor,
        bInheritHandle: 0,
    };
    let success = unsafe { CreateDirectoryW(path.as_ptr(), &attributes) };
    let error = std::io::Error::last_os_error();
    unsafe {
        LocalFree(descriptor);
    }
    if success == 0 {
        Err(error.to_string())
    } else {
        Ok(())
    }
}
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
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
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
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
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
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
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
struct Endpoint {
    port: u16,
    pid: u32,
    process: Option<Handle>,
}
impl Endpoint {
    #[cfg_attr(
        test,
        expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
    )]
    fn exited(&self) -> bool {
        match &self.process {
            Some(handle) => unsafe { WaitForSingleObject(handle.0, 0) == WAIT_OBJECT_0 },
            None => vrchat_pids().is_ok_and(|pids| !pids.contains(&self.pid)),
        }
    }
}
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
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
#[cfg_attr(
    test,
    expect(dead_code, reason = "Live VRChat monitoring is disabled in unit tests")
)]
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

#[path = "startup.rs"]
mod startup;
#[cfg(test)]
use startup::startup_command;
pub use startup::{
    executable_version, installation_directory, installed_version, migrate_startup,
    registered_directory, restore_startup, set_startup, startup_available, startup_enabled,
    startup_snapshot, uninstall_cleanup,
};

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
pub fn normal_path(path: &std::path::Path) -> Result<String> {
    let text = path.to_str().ok_or("Path must be valid Unicode")?;
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        Ok(format!(r"\\{rest}"))
    } else if let Some(rest) = text.strip_prefix(r"\\?\") {
        Ok(rest.to_owned())
    } else {
        Ok(text.to_owned())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn instance_mutex_blocks_duplicates_and_releases_on_drop() {
        let sid = current_user_id().unwrap();
        let name = format!("Local\\Alcnaka.AlcMute.Tests.{sid}.{}", std::process::id());
        let first = SingleInstance::named(&name).unwrap().unwrap();
        assert!(SingleInstance::named(&name).unwrap().is_none());
        drop(first);
        assert!(SingleInstance::named(&name).unwrap().is_some());
    }
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
            "\"C:\\Apps\\mute.exe\" --background --data-dir \"D:\\My Settings\""
        );
        assert!(!command.contains("--test-mode"));
    }
}
