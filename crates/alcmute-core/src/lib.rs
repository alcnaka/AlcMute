use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
pub mod appearance;
mod apply_error;
mod migration;
pub use apply_error::{ApplyError, ApplyErrorKind};
pub mod settings_store;
mod shortcut;
#[cfg(feature = "windows-integration")]
pub mod updater;
use rosc::{OscMessage, OscPacket, OscType};
use serde::{Deserialize, Serialize};
use shortcut::{GlobalHotKeyManager, HotKey};
use std::{
    fs,
    net::{SocketAddr, UdpSocket},
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex, OnceLock,
    },
    thread::{self, JoinHandle},
};

pub type Result<T> = std::result::Result<T, String>;
#[cfg(feature = "windows-integration")]
pub mod windows;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    #[serde(skip_serializing_if = "default_target")]
    pub target: String,
    #[serde(skip_serializing_if = "default_listen")]
    pub listen: String,
    #[serde(skip_serializing_if = "default_shortcut")]
    pub shortcut: String,
    #[serde(skip_serializing)]
    pub autostart: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub tray_double_click_settings: bool,
    #[serde(skip_serializing_if = "is_true")]
    pub show_settings_on_startup: bool,
    #[serde(skip_serializing_if = "is_true")]
    pub auto_check_updates: bool,
    #[serde(skip_serializing_if = "default_theme")]
    pub theme: appearance::Theme,
}
fn default_theme(value: &appearance::Theme) -> bool {
    *value == appearance::Theme::Auto
}
fn default_target(value: &String) -> bool {
    value == &Config::default().target
}
fn default_listen(value: &String) -> bool {
    value == &Config::default().listen
}
fn default_shortcut(value: &String) -> bool {
    value == &Config::default().shortcut
}
fn is_false(value: &bool) -> bool {
    !value
}
fn is_true(value: &bool) -> bool {
    *value
}
impl Default for Config {
    fn default() -> Self {
        Self {
            target: "127.0.0.1:9000".into(),
            listen: "127.0.0.1:9001".into(),
            shortcut: "Ctrl+Alt+KeyM".into(),
            autostart: false,
            tray_double_click_settings: false,
            show_settings_on_startup: true,
            auto_check_updates: true,
            theme: appearance::Theme::Auto,
        }
    }
}
impl Config {
    pub fn load(options: &Options) -> Result<Self> {
        let path = options.data_dir.join("settings.toml");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
            Err(error) => return Err(format!("Cannot read {}: {error}", path.display())),
        };
        let mut config: Self = toml::from_str(&text)
            .map_err(|error| format!("Invalid settings in {}: {error}", path.display()))?;
        let legacy = options.data_dir.join("updates.toml");
        if legacy.exists() {
            let document: toml::Value = toml::from_str(&text).map_err(|error| error.to_string())?;
            if document.get("auto_check_updates").is_none() {
                #[derive(Deserialize)]
                #[serde(default, deny_unknown_fields)]
                struct Legacy {
                    auto_check: bool,
                }
                impl Default for Legacy {
                    fn default() -> Self {
                        Self { auto_check: true }
                    }
                }
                let old: Legacy = toml::from_str(
                    &fs::read_to_string(&legacy).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("更新確認設定を移行できません: {error}"))?;
                config.auto_check_updates = old.auto_check;
            }
            config.validate()?;
            let pending = options.data_dir.join("settings.pending.toml");
            let mut unified = toml::Value::try_from(&config).map_err(|error| error.to_string())?;
            // Keep an explicit precedence marker until the legacy file is archived.
            unified
                .as_table_mut()
                .ok_or("Invalid settings table")?
                .insert(
                    "auto_check_updates".into(),
                    toml::Value::Boolean(config.auto_check_updates),
                );
            settings_store::write_pending(
                &pending,
                toml::to_string(&unified)
                    .map_err(|error| error.to_string())?
                    .as_bytes(),
            )?;
            if path.exists() {
                settings_store::backup(&path)?;
            }
            settings_store::replace(&pending, &path)?;
            // Retain legacy data as an archive, never as a second active settings file.
            let mut backup = options.data_dir.join("updates.toml.bak");
            if backup.exists() {
                backup = options.data_dir.join(format!(
                    "updates.{}.bak",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|error| error.to_string())?
                        .as_nanos()
                ));
            }
            fs::rename(&legacy, backup)
                .map_err(|error| format!("旧更新設定を保全できません: {error}"))?;
        }
        config.validate()?;
        Ok(config)
    }
    pub fn validate(&self) -> Result<()> {
        for addr in [&self.target, &self.listen] {
            let parsed: SocketAddr = addr
                .parse()
                .map_err(|_| "OSC address must be an IP address and port".to_string())?;
            if !parsed.ip().is_loopback() || !parsed.is_ipv4() || parsed.port() == 0 {
                return Err("Use an IPv4 loopback address with port 1-65535".into());
            }
        }
        if self.target == self.listen {
            return Err("Send and receive addresses must differ".into());
        }
        let _: HotKey = self
            .shortcut
            .parse()
            .map_err(|e| format!("Invalid shortcut: {e}"))?;
        Ok(())
    }
}
#[derive(Clone, Debug, Default)]
pub struct Status {
    pub muted: Option<bool>,
    pub detail: String,
    pub sequence: u64,
    pub connection: Connection,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Connection {
    #[default]
    Checking,
    Connected,
    Disconnected,
    Unavailable,
}
impl Status {
    pub fn text(&self) -> String {
        if self.connection == Connection::Disconnected {
            return "VRChat is not running".into();
        }
        if self.connection == Connection::Unavailable {
            return "Unknown (OSCQuery unavailable)".into();
        }
        let state = match self.muted {
            Some(true) => "Last OSC: Muted",
            Some(false) => "Last OSC: Unmuted",
            None => "Unknown (waiting for OSC)",
        };
        if self.detail.is_empty() {
            state.into()
        } else {
            format!("{state} | {}", self.detail)
        }
    }
}
#[derive(Clone, Copy, Debug)]
pub enum Event {
    Status,
    Show,
    ShowExistingInstance,
    Hide,
    Toggle,
    ExerciseSettings,
    ExerciseAutostart,
    Quit,
}
pub type Notify = Arc<dyn Fn(Event) + Send + Sync>;

#[derive(Clone)]
pub struct Options {
    pub data_dir: PathBuf,
    pub benchmark: bool,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            data_dir: migration::default_directory(),
            benchmark: false,
        }
    }
}
impl Options {
    pub fn from_args() -> Result<Self> {
        Self::parse_args(std::env::args().skip(1))
    }
    fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Self> {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--background" | "--install-migrate" | "--uninstall-cleanup" => {}
                "--test-mode" if cfg!(feature = "dev-tools") => options.benchmark = true,
                "--data-dir" => {
                    options.data_dir =
                        PathBuf::from(args.next().ok_or("--data-dir requires a path")?)
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
        }
        options.data_dir = std::path::absolute(&options.data_dir).map_err(|e| e.to_string())?;
        // Old sign-in commands explicitly pass the former default directory.
        if !options.benchmark && options.data_dir == migration::legacy_directory() {
            options.data_dir = migration::default_directory();
        }
        Ok(options)
    }
    pub fn marker(&self, value: &str) {
        if self.benchmark {
            let _ = fs::write(self.data_dir.join("ui-state.txt"), value);
        }
    }
    pub fn install_panic_log(&self) {
        let options = self.clone();
        std::panic::set_hook(Box::new(move |info| {
            options.log(&info.to_string());
        }));
    }
    pub fn log(&self, error: &str) {
        let _ = self.write_log(error);
    }
    pub fn write_log(&self, error: &str) -> Result<PathBuf> {
        fs::create_dir_all(&self.data_dir).map_err(|e| e.to_string())?;
        let path = self.data_dir.join("error.log");
        fs::write(&path, error).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

pub fn encode(address: &str, args: Vec<OscType>) -> Result<Vec<u8>> {
    rosc::encoder::encode(&OscPacket::Message(OscMessage {
        addr: address.into(),
        args,
    }))
    .map_err(|e| e.to_string())
}
pub fn mute_value(packet: &OscPacket) -> Option<bool> {
    match packet {
        OscPacket::Message(message) if message.addr == "/avatar/parameters/MuteSelf" => {
            match message.args.first() {
                Some(OscType::Bool(value)) => Some(*value),
                _ => None,
            }
        }
        OscPacket::Bundle(bundle) => bundle.content.iter().filter_map(mute_value).next_back(),
        _ => None,
    }
}
pub fn voice_pulse(socket: &UdpSocket, target: &str) -> Result<()> {
    // Always release the OSC button, including when sending the press fails.
    let press = socket.send_to(&encode("/input/Voice", vec![OscType::Int(1)])?, target);
    // Separate frames so the receiver can observe the input edge.
    thread::sleep(std::time::Duration::from_millis(40));
    let release = socket.send_to(&encode("/input/Voice", vec![OscType::Int(0)])?, target);
    // A queued press must not immediately overwrite this release in the same
    // VRChat frame. Keep a release interval as well as a press interval.
    thread::sleep(std::time::Duration::from_millis(40));
    press.map_err(|e| e.to_string())?;
    release.map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Debug)]
enum Command {
    Toggle,
    Refresh,
    Stop,
}
const COMMAND_CAPACITY: usize = 32;
#[derive(Clone)]
struct CommandSender {
    tx: mpsc::SyncSender<Command>,
    status: Arc<Mutex<Status>>,
    notify: Notify,
}
impl CommandSender {
    // Never wait for the OSC worker on the UI or keyboard-hook thread.
    fn send(&self, command: Command) -> std::result::Result<(), mpsc::TrySendError<Command>> {
        let result = self.tx.try_send(command);
        if let Err(mpsc::TrySendError::Full(Command::Toggle)) = &result {
            self.status.lock().unwrap().detail =
                "切り替え要求が多すぎます。少し待ってから操作してください".into();
            (self.notify)(Event::Status);
        }
        result
    }
}
struct Runtime {
    tx: CommandSender,
    stop: Arc<AtomicBool>,
    address: SocketAddr,
    threads: Vec<JoinHandle<()>>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        let _ = self.tx.send(Command::Stop);
        if let Ok(wake) = UdpSocket::bind("127.0.0.1:0") {
            let _ = wake.send_to(&[], self.address);
        }
        for handle in self.threads.drain(..) {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}
fn runtime(
    config: &Config,
    status: Arc<Mutex<Status>>,
    notify: Notify,
    options: &Options,
) -> Result<Runtime> {
    let benchmark = options.benchmark;
    let capture_dir = options.data_dir.clone();
    let receive = UdpSocket::bind(&config.listen).map_err(|e| {
        format!(
            "Cannot bind {}: {e}. Another OSC app may be using this port.",
            config.listen
        )
    })?;
    let address = receive.local_addr().map_err(|e| e.to_string())?;
    let send = UdpSocket::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let (tx, rx) = mpsc::sync_channel(COMMAND_CAPACITY);
    let tx = CommandSender {
        tx,
        status: status.clone(),
        notify: notify.clone(),
    };
    let stop = Arc::new(AtomicBool::new(false));
    let command_stop = stop.clone();
    let target = config.target.clone();
    let command_status = status.clone();
    let command_notify = notify.clone();
    let command_dir = options.data_dir.clone();
    let commands = thread::spawn(move || {
        let mut toggle_count = 0u64;
        while let Ok(command) = rx.recv() {
            if command_stop.load(Ordering::Acquire) {
                break;
            }
            match command {
                Command::Stop => break,
                Command::Refresh => {
                    command_notify(Event::Status);
                    continue;
                }
                Command::Toggle => {}
            }
            if benchmark {
                toggle_count += 1;
                let _ = fs::write(
                    command_dir.join("toggle-count.txt"),
                    toggle_count.to_string(),
                );
            }
            let sequence = command_status.lock().unwrap().sequence;
            let result = voice_pulse(&send, &target);
            let mut state = command_status.lock().unwrap();
            state.detail = match result {
                Ok(()) if state.sequence == sequence => {
                    "Toggle sent; awaiting VRChat confirmation".into()
                }
                Ok(()) => String::new(),
                Err(e) => format!("Send failed: {e}"),
            };
            drop(state);
            command_notify(Event::Status);
        }
    });
    let receive_stop = stop.clone();
    #[cfg(all(feature = "windows-integration", not(test)))]
    let monitor =
        windows::spawn_monitor(config.clone(), status.clone(), notify.clone(), stop.clone());
    let receiver = thread::spawn(move || {
        let mut bytes = [0u8; 65536];
        loop {
            let result = receive.recv_from(&mut bytes);
            if receive_stop.load(Ordering::Acquire) {
                break;
            }
            match result {
                Ok((count, source)) if source.ip().is_loopback() => {
                    if let Ok((_, packet)) = rosc::decoder::decode_udp(&bytes[..count]) {
                        #[cfg(feature = "dev-tools")]
                        if benchmark {
                            if let OscPacket::Message(ref message) = packet {
                                let action = match message.addr.as_str() {
                                    "/alcmute/test/show" => Some(Event::Show),
                                    "/alcmute/test/hide" => Some(Event::Hide),
                                    "/alcmute/test/toggle" => Some(Event::Toggle),
                                    "/alcmute/test/save-test" => Some(Event::ExerciseSettings),
                                    "/alcmute/test/autostart-test" => {
                                        Some(Event::ExerciseAutostart)
                                    }
                                    "/alcmute/test/quit" => Some(Event::Quit),
                                    _ => None,
                                };
                                if let Some(action) = action {
                                    notify(action);
                                    continue;
                                }
                            }
                        }
                        if let Some(muted) = mute_value(&packet) {
                            let mut state = status.lock().unwrap();
                            let changed = state.muted != Some(muted) || !state.detail.is_empty();
                            state.muted = Some(muted);
                            state.connection = Connection::Connected;
                            state.sequence += 1;
                            state.detail.clear();
                            drop(state);
                            if benchmark {
                                let _ =
                                    fs::write(capture_dir.join("osc-state.txt"), muted.to_string());
                                if let Ok(mut log) = fs::OpenOptions::new()
                                    .create(true)
                                    .append(true)
                                    .open(capture_dir.join("osc-events.csv"))
                                {
                                    use std::io::Write;
                                    let at = std::time::SystemTime::now()
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_millis();
                                    let _ = writeln!(log, "{at},{muted}");
                                }
                            }
                            if changed {
                                notify(Event::Status);
                            }
                        }
                    }
                }
                Ok(_) => {}
                Err(e) => {
                    status.lock().unwrap().detail = format!("OSC receive failed: {e}");
                    notify(Event::Status);
                    break;
                }
            }
        }
    });
    #[allow(unused_mut)]
    let mut threads = vec![commands, receiver];
    #[cfg(all(feature = "windows-integration", not(test)))]
    threads.push(monitor);
    Ok(Runtime {
        tx,
        stop,
        address,
        threads,
    })
}

pub struct Controller {
    pub config: Config,
    pub options: Options,
    manager: GlobalHotKeyManager,
    hotkey: HotKey,
    hotkey_registered: bool,
    shortcut_warning: Option<String>,
    runtime: Option<Runtime>,
    status: Arc<Mutex<Status>>,
    notify: Notify,
    orphan_hotkeys: Vec<HotKey>,
    #[cfg(test)]
    faults: ApplyFaults,
}
#[cfg(test)]
#[derive(Default)]
struct ApplyFaults {
    register: std::cell::Cell<bool>,
    register_recovery: std::cell::Cell<bool>,
    unregister: std::cell::Cell<bool>,
    runtime: std::cell::Cell<bool>,
    runtime_recovery: std::cell::Cell<bool>,
}
impl Controller {
    pub fn new(options: Options, notify: Notify) -> Result<Self> {
        let migrate = !options.benchmark && options.data_dir == migration::default_directory();
        if migrate {
            migration::settings(&migration::legacy_directory(), &options.data_dir)?;
        }
        fs::create_dir_all(&options.data_dir).map_err(|e| e.to_string())?;
        #[allow(unused_mut)]
        let mut config = Config::load(&options)?;
        config.validate()?;
        #[cfg(feature = "windows-integration")]
        {
            config.autostart = windows::startup_enabled(&options)?;
        }
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let hotkey: HotKey = config.shortcut.parse()?;
        let shortcut_warning = manager.register(hotkey).err().map(|error| {
            format!("ショートカットを登録できないため無効です。別のキーに変更して保存してください: {error}")
        });
        if let Some(warning) = &shortcut_warning {
            options.log(warning);
        }
        let hotkey_registered = shortcut_warning.is_none();
        let status = Arc::new(Mutex::new(Status::default()));
        let runtime = runtime(&config, status.clone(), notify.clone(), &options)?;
        if hotkey_registered {
            install_handler(&runtime, hotkey);
        }
        Ok(Self {
            config,
            options,
            manager,
            hotkey,
            hotkey_registered,
            shortcut_warning,
            runtime: Some(runtime),
            status,
            notify,
            orphan_hotkeys: Vec::new(),
            #[cfg(test)]
            faults: ApplyFaults::default(),
        })
    }
    pub fn shortcut_warning(&self) -> Option<&str> {
        self.shortcut_warning.as_deref()
    }
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
    pub fn toggle(&self) {
        if let Some(runtime) = &self.runtime {
            let _ = runtime.tx.send(Command::Toggle);
        }
    }
    /// Sending OSC does not require a successful OSCQuery state lookup.
    pub fn can_toggle(&self) -> bool {
        self.runtime.is_some()
    }
    pub fn apply(&mut self, mut config: Config) -> std::result::Result<(), ApplyError> {
        #[cfg(feature = "windows-integration")]
        if !windows::startup_available(&self.options) {
            config.autostart = false;
        }
        config.validate()?;
        let hotkey: HotKey = config.shortcut.parse().map_err(ApplyError::from)?;
        let changed = hotkey != self.hotkey;
        let needs_registration = changed || !self.hotkey_registered;
        let network_changed = config.target != self.config.target
            || config.listen != self.config.listen
            || self.runtime.is_none();
        let temp = self.options.data_dir.join("settings.pending.toml");
        let serialized = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
        if let Err(error) = settings_store::write_pending(&temp, serialized.as_bytes()) {
            let mut failure = ApplyError::from(format!("Cannot prepare settings: {error}"));
            cleanup_pending(&temp, &mut failure);
            return Err(failure);
        }
        let mut registered = false;
        let mut retired_old = false;
        let mut stopped_old = false;
        let mut candidate = None;
        let candidate_status = Arc::new(Mutex::new(Status::default()));
        let result = (|| -> std::result::Result<(), ApplyError> {
            // Retry cleanup after a previous unregister failure; never lose track
            // of a native registration which still exists.
            while let Some(key) = self.orphan_hotkeys.last().copied() {
                self.manager.unregister(key)?;
                self.orphan_hotkeys.pop();
            }
            if needs_registration {
                self.register_shortcut(hotkey, false)?;
                registered = true;
            }
            if network_changed {
                if self.runtime.is_some() && config.listen == self.config.listen {
                    self.runtime.take();
                    stopped_old = true;
                }
                candidate = Some(self.create_runtime(&config, candidate_status.clone(), false)?);
            }
            if changed && self.hotkey_registered {
                self.unregister_shortcut(self.hotkey)?;
                self.hotkey_registered = false;
                retired_old = true;
            }
            commit_settings(&self.options, &config, &temp)?;
            Ok(())
        })();
        if let Err(mut error) = result {
            drop(candidate);
            let mut failures = Vec::new();
            if retired_old {
                match self.register_shortcut(self.hotkey, true) {
                    Ok(()) => self.hotkey_registered = true,
                    Err(e) => failures.push(format!("Shortcut recovery failed: {e}")),
                }
            }
            if registered {
                if let Err(e) = self.manager.unregister(hotkey) {
                    self.orphan_hotkeys.push(hotkey);
                    failures.push(format!("New shortcut cleanup failed: {e}"));
                }
            }
            if stopped_old {
                match self.create_runtime(&self.config.clone(), self.status.clone(), true) {
                    Ok(value) => self.runtime = Some(value),
                    Err(e) => failures.push(format!("OSC runtime recovery failed: {e}")),
                }
            }
            if !failures.is_empty() {
                error.kind = ApplyErrorKind::RecoveryFailed;
                error
                    .message
                    .push_str(&format!("; {}", failures.join("; ")));
            }
            if !self.orphan_hotkeys.is_empty() {
                error.kind = ApplyErrorKind::RecoveryFailed;
            }
            if self.runtime.is_none() {
                error.kind = ApplyErrorKind::RecoveryFailed;
                let mut state = self.status.lock().unwrap();
                state.muted = None;
                state.connection = Connection::Unavailable;
                state.detail =
                    "OSC通信が停止しています。設定を再保存するか再起動してください".into();
            }
            if !self.hotkey_registered {
                error.kind = ApplyErrorKind::RecoveryFailed;
                self.status.lock().unwrap().detail =
                    "ショートカットが停止しています。設定を再保存するか再起動してください".into();
            }
            if error.kind == ApplyErrorKind::RecoveryFailed {
                #[cfg(feature = "windows-integration")]
                match windows::startup_enabled(&self.options) {
                    Ok(enabled) => self.config.autostart = enabled,
                    Err(e) => error
                        .message
                        .push_str(&format!("; Startup state could not be read: {e}")),
                }
            }
            cleanup_pending(&temp, &mut error);
            if let Some(old) = &self.runtime {
                if self.hotkey_registered {
                    install_handler(old, self.hotkey);
                } else if let Some(route) = HOTKEY_ROUTE.get() {
                    route.lock().unwrap().take();
                }
                let _ = old.tx.send(Command::Refresh);
            } else if let Some(route) = HOTKEY_ROUTE.get() {
                route.lock().unwrap().take();
                (self.notify)(Event::Status);
            }
            if error.kind == ApplyErrorKind::Rejected
                && self.runtime.is_some()
                && self.hotkey_registered
            {
                error.message.push_str("; 従来の設定で動作しています");
            }
            return Err(error);
        }
        if let Some(candidate) = candidate {
            self.runtime = Some(candidate);
            self.status = candidate_status;
        }
        self.shortcut_warning = None;
        self.config = config;
        self.hotkey = hotkey;
        self.hotkey_registered = true;
        if let Some(runtime) = &self.runtime {
            install_handler(runtime, hotkey);
            let _ = runtime.tx.send(Command::Refresh);
        }
        if let Err(error) = fs::remove_file(&temp) {
            if error.kind() != std::io::ErrorKind::NotFound {
                self.options.log(&format!(
                    "Settings saved, but pending file cleanup failed: {error}"
                ));
            }
        }
        Ok(())
    }
    fn register_shortcut(&self, key: HotKey, _recovery: bool) -> Result<()> {
        #[cfg(test)]
        if self.faults.register.get() || (_recovery && self.faults.register_recovery.get()) {
            return Err("Injected shortcut registration failure".into());
        }
        self.manager
            .register(key)
            .map_err(|e| format!("Shortcut registration failed: {e}"))
    }
    fn unregister_shortcut(&self, key: HotKey) -> Result<()> {
        #[cfg(test)]
        if self.faults.unregister.get() {
            return Err("Injected shortcut removal failure".into());
        }
        self.manager
            .unregister(key)
            .map_err(|e| format!("Shortcut removal failed: {e}"))
    }
    fn create_runtime(
        &self,
        config: &Config,
        status: Arc<Mutex<Status>>,
        _recovery: bool,
    ) -> Result<Runtime> {
        #[cfg(test)]
        if (_recovery && self.faults.runtime_recovery.get())
            || (!_recovery && self.faults.runtime.get())
        {
            return Err("Injected OSC runtime failure".into());
        }
        runtime(config, status, self.notify.clone(), &self.options)
    }
    pub fn reset_to_defaults(&mut self) -> Result<()> {
        self.apply(Config::default()).map_err(|e| e.to_string())
    }
    pub fn begin_shortcut_capture(&mut self) -> Result<()> {
        self.manager.begin_capture()?;
        if self.hotkey_registered {
            if let Err(error) = self.manager.unregister(self.hotkey) {
                self.manager.end_capture();
                return Err(error);
            }
            self.hotkey_registered = false;
        }
        if let Some(route) = HOTKEY_ROUTE.get() {
            route.lock().unwrap().take();
        }
        Ok(())
    }
    pub fn end_shortcut_capture(&mut self) -> Result<()> {
        self.manager.end_capture();
        if !self.hotkey_registered && self.shortcut_warning.is_none() {
            self.manager
                .register(self.hotkey)
                .map_err(|e| format!("ショートカットの再登録に失敗しました: {e}"))?;
            self.hotkey_registered = true;
        }
        if self.hotkey_registered {
            if let Some(runtime) = &self.runtime {
                install_handler(runtime, self.hotkey);
            }
        }
        Ok(())
    }
}
fn cleanup_pending(path: &std::path::Path, error: &mut ApplyError) {
    if let Err(cleanup) = fs::remove_file(path) {
        if cleanup.kind() != std::io::ErrorKind::NotFound {
            error
                .message
                .push_str(&format!("; Pending file cleanup failed: {cleanup}"));
        }
    }
}

fn commit_settings(
    options: &Options,
    config: &Config,
    temp: &std::path::Path,
) -> std::result::Result<(), ApplyError> {
    #[cfg(feature = "windows-integration")]
    if !windows::startup_available(options) {
        return commit_settings_with(options, config, temp, || Ok(()), || Ok(()));
    }
    #[cfg(feature = "windows-integration")]
    let previous = windows::startup_snapshot(options)?;
    commit_settings_with(
        options,
        config,
        temp,
        || {
            #[cfg(feature = "windows-integration")]
            windows::set_startup(options, config.autostart)?;
            Ok(())
        },
        || {
            #[cfg(feature = "windows-integration")]
            windows::restore_startup(options, previous.as_deref())?;
            Ok(())
        },
    )
}

fn commit_settings_with(
    options: &Options,
    config: &Config,
    temp: &std::path::Path,
    update_startup: impl FnOnce() -> Result<()>,
    restore_startup: impl FnOnce() -> Result<()>,
) -> std::result::Result<(), ApplyError> {
    update_startup()
        .map_err(|e| ApplyError::from(format!("Cannot update sign-in startup: {e}")))?;
    let path = options.data_dir.join("settings.toml");
    let result = (|| -> std::io::Result<()> {
        if path.is_file() {
            settings_store::backup(&path).map_err(std::io::Error::other)?;
        }
        if config == &Config::default() {
            match fs::remove_file(&path) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
                result => result,
            }
        } else {
            settings_store::replace(temp, &path).map_err(std::io::Error::other)
        }
    })();
    if let Err(error) = result {
        if let Err(rollback) = restore_startup() {
            return Err(ApplyError::recovery(format!(
                "Cannot save settings: {error}; startup rollback failed: {rollback}"
            )));
        }
        return Err(format!("Cannot save settings: {error}").into());
    }
    let _ = fs::remove_file(temp);
    Ok(())
}
// global-hotkey's handler is a OnceCell: install once, route through a mutable slot.
type HotkeyRoute = Option<(u32, CommandSender)>;
static HOTKEY_ROUTE: OnceLock<Mutex<HotkeyRoute>> = OnceLock::new();
fn install_handler(runtime: &Runtime, hotkey: HotKey) {
    let route = HOTKEY_ROUTE.get_or_init(|| {
        GlobalHotKeyEvent::set_event_handler(Some(move |event: GlobalHotKeyEvent| {
            if event.state == HotKeyState::Pressed {
                if let Some(route) = HOTKEY_ROUTE.get() {
                    if let Some((id, sender)) = route.lock().unwrap().as_ref() {
                        if *id == event.id {
                            let _ = sender.send(Command::Toggle);
                        }
                    }
                }
            }
        }));
        Mutex::new(None)
    });
    *route.lock().unwrap() = Some((hotkey.id(), runtime.tx.clone()));
}
impl Drop for Controller {
    fn drop(&mut self) {
        if self.hotkey_registered {
            let _ = self.manager.unregister(self.hotkey);
        }
        for key in &self.orphan_hotkeys {
            if let Err(error) = self.manager.unregister(*key) {
                self.options.log(&error);
            }
        }
        // Clear the captured command sender before joining the workers.
        if let Some(route) = HOTKEY_ROUTE.get() {
            route.lock().unwrap().take();
        }
        self.runtime.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn toggle_queue_is_bounded_and_preserves_normal_bursts() {
        let (tx, rx) = mpsc::sync_channel(COMMAND_CAPACITY);
        let status = Arc::new(Mutex::new(Status::default()));
        let sender = CommandSender {
            tx,
            status: status.clone(),
            notify: Arc::new(|_| {}),
        };
        for _ in 0..COMMAND_CAPACITY {
            sender.send(Command::Toggle).unwrap();
        }
        assert!(matches!(
            sender.send(Command::Toggle),
            Err(mpsc::TrySendError::Full(_))
        ));
        assert!(!status.lock().unwrap().detail.is_empty());
        for _ in 0..COMMAND_CAPACITY {
            assert!(matches!(rx.try_recv().unwrap(), Command::Toggle));
        }
        assert!(matches!(rx.try_recv(), Err(mpsc::TryRecvError::Empty)));
    }

    #[test]
    fn shutdown_does_not_wait_for_a_full_toggle_queue() {
        let target = UdpSocket::bind("127.0.0.1:0").unwrap();
        let config = Config {
            listen: "127.0.0.1:0".into(),
            target: target.local_addr().unwrap().to_string(),
            ..Default::default()
        };
        let options = Options {
            data_dir: std::env::temp_dir(),
            benchmark: false,
        };
        let worker = runtime(
            &config,
            Arc::new(Mutex::new(Status::default())),
            Arc::new(|_| {}),
            &options,
        )
        .unwrap();
        for _ in 0..COMMAND_CAPACITY {
            let _ = worker.tx.send(Command::Toggle);
        }
        let start = std::time::Instant::now();
        drop(worker);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }

    #[test]
    fn test_mode_requires_explicit_development_feature() {
        assert_eq!(
            Options::parse_args(["--test-mode".into()]).is_ok(),
            cfg!(feature = "dev-tools")
        );
    }
    #[test]
    fn invalid_arguments_and_corrupt_settings_fail_without_overwriting_data() {
        assert!(Options::parse_args(["--data-dir".into()]).is_err());
        assert!(Options::parse_args(["--unknown".into()]).is_err());
        let dir = std::env::temp_dir().join(format!("alcmute-corrupt-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.toml");
        let bad = "shortcut = [corrupt TOML";
        fs::write(&path, bad).unwrap();
        let options = Options {
            data_dir: dir.clone(),
            benchmark: true,
        };
        let result = Controller::new(options.clone(), Arc::new(|_| {}));
        assert!(result.err().unwrap().contains("Invalid settings"));
        assert_eq!(fs::read_to_string(&path).unwrap(), bad);
        fs::remove_file(&path).unwrap();
        fs::remove_dir(&dir).unwrap();
        fs::write(&dir, "not a directory").unwrap();
        assert!(options.write_log("test").is_err());
        options.log("test"); // best-effort logging must not panic
        fs::remove_file(dir).unwrap();
    }
    #[test]
    fn conflicting_shortcut_starts_without_registration_and_can_be_changed() {
        let dir = std::env::temp_dir().join(format!("alcmute-conflict-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let config = Config {
            listen: free_address(),
            target: free_address(),
            shortcut: "Ctrl+Alt+Shift+F20".into(),
            ..Default::default()
        };
        let owner = GlobalHotKeyManager::new().unwrap();
        let key = config.shortcut.parse().unwrap();
        owner.register(key).unwrap();
        fs::write(dir.join("settings.toml"), toml::to_string(&config).unwrap()).unwrap();
        let mut core = Controller::new(
            Options {
                data_dir: dir.clone(),
                benchmark: true,
            },
            Arc::new(|_| {}),
        )
        .unwrap();
        assert!(!core.hotkey_registered);
        assert!(core.shortcut_warning().is_some());
        assert!(core.can_toggle());
        core.begin_shortcut_capture().unwrap();
        core.end_shortcut_capture().unwrap();
        assert!(!core.hotkey_registered);
        let mut changed = config;
        changed.shortcut = "Ctrl+Alt+Shift+F19".into();
        core.apply(changed).unwrap();
        assert!(core.hotkey_registered);
        assert!(core.shortcut_warning().is_none());
        drop(core);
        owner.unregister(key).unwrap();
        fs::remove_dir_all(dir).unwrap();
    }

    #[cfg(all(feature = "windows-integration", not(feature = "installer")))]
    #[test]
    fn portable_save_and_reset_never_change_startup_even_with_legacy_true() {
        let dir =
            std::env::temp_dir().join(format!("alcmute-portable-policy-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let options = Options {
            data_dir: dir.clone(),
            benchmark: false,
        };
        let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER)
            .create_subkey(format!(
                r"Software\AlcMute\Testing\{}\Run",
                std::process::id()
            ))
            .unwrap();
        let previous = key.get_raw_value("AlcMute").ok();
        key.set_value("AlcMute", &"legacy sentinel").unwrap();
        let config = Config {
            target: free_address(),
            listen: free_address(),
            shortcut: "Ctrl+Alt+Shift+F18".into(),
            autostart: true,
            ..Default::default()
        };
        let mut text = toml::to_string(&config).unwrap();
        text.push_str("autostart = true\n");
        fs::write(dir.join("settings.toml"), text).unwrap();
        let mut core = Controller::new(options.clone(), Arc::new(|_| {})).unwrap();
        assert!(!core.config.autostart);
        let mut draft = core.config.clone();
        draft.autostart = true;
        core.apply(draft).unwrap();
        assert!(!core.config.autostart);
        assert!(!fs::read_to_string(dir.join("settings.toml"))
            .unwrap()
            .contains("autostart"));
        drop(core);
        let pending = dir.join("settings.pending.toml");
        fs::write(&pending, "").unwrap();
        commit_settings(&options, &Config::default(), &pending).unwrap();
        assert!(!dir.join("settings.toml").exists());
        assert_eq!(
            key.get_value::<String, _>("AlcMute").unwrap(),
            "legacy sentinel"
        );
        if let Some(value) = previous {
            key.set_raw_value("AlcMute", &value).unwrap();
        } else {
            key.delete_value("AlcMute").unwrap();
        }
        fs::remove_dir_all(dir).unwrap();
    }
    fn free_address() -> String {
        UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .to_string()
    }

    #[test]
    fn apply_preserves_runtime_and_recovers_from_network_storage_and_hotkey_failures() {
        let dir = std::env::temp_dir().join(format!("alcmute-apply-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let target = UdpSocket::bind("127.0.0.1:0").unwrap();
        target
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let config = Config {
            listen: free_address(),
            target: target.local_addr().unwrap().to_string(),
            shortcut: "Ctrl+Alt+Shift+F23".into(),
            ..Default::default()
        };
        let settings = dir.join("settings.toml");
        fs::write(&settings, toml::to_string(&config).unwrap()).unwrap();
        let options = Options {
            data_dir: dir.clone(),
            benchmark: true,
        };
        let mut core = Controller::new(options, Arc::new(|_| {})).unwrap();
        assert_eq!(core.status().muted, None);
        assert!(core.can_toggle());
        core.toggle();
        for expected in [1, 0] {
            let mut bytes = [0; 256];
            let count = target.recv(&mut bytes).unwrap();
            let (_, packet) = rosc::decoder::decode_udp(&bytes[..count]).unwrap();
            let OscPacket::Message(message) = packet else {
                panic!("Expected a voice pulse")
            };
            assert_eq!(message.args, vec![OscType::Int(expected)]);
        }
        assert_eq!(
            core.status().muted,
            None,
            "sending must not invent a confirmed mute state"
        );
        let id = core.runtime.as_ref().unwrap().threads[0].thread().id();
        core.status.lock().unwrap().muted = Some(true);
        let mut update = core.config.clone();
        update.tray_double_click_settings = true;
        update.show_settings_on_startup = false;
        update.theme = appearance::Theme::Dark;
        update.auto_check_updates = false;
        core.apply(update.clone()).unwrap();
        core.apply(update.clone()).unwrap(); // unchanged apply keeps the runtime
        update.shortcut = "Ctrl+Alt+Shift+F22".into();
        core.apply(update.clone()).unwrap();
        assert_eq!(core.runtime.as_ref().unwrap().threads[0].thread().id(), id);
        assert_eq!(core.status().muted, Some(true));

        let blocked = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut network = update.clone();
        network.listen = blocked.local_addr().unwrap().to_string();
        let error = core.apply(network).unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::Rejected);
        assert_eq!(core.config, update);
        assert_eq!(core.runtime.as_ref().unwrap().threads[0].thread().id(), id);
        assert_eq!(core.status().muted, Some(true));
        assert!(!dir.join("settings.pending.toml").exists());

        let mut shortcut = update.clone();
        shortcut.shortcut = "Ctrl+Alt+Shift+F21".into();
        shortcut.theme = appearance::Theme::Light;
        shortcut.auto_check_updates = true;
        core.faults.register.set(true);
        assert!(core.apply(shortcut.clone()).is_err());
        core.faults.register.set(false);
        core.faults.unregister.set(true);
        assert!(core.apply(shortcut.clone()).is_err());
        core.faults.unregister.set(false);
        assert_eq!(core.config, update);
        assert!(core.hotkey_registered);
        assert!(!dir.join("settings.pending.toml").exists());

        fs::remove_file(&settings).unwrap();
        fs::create_dir(&settings).unwrap(); // deterministic rename failure
        assert!(core.apply(shortcut.clone()).is_err());
        assert!(core.hotkey_registered);
        assert_eq!(core.config, update);
        assert_eq!(core.runtime.as_ref().unwrap().threads[0].thread().id(), id);
        core.faults.register_recovery.set(true);
        let error = core.apply(shortcut).unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::RecoveryFailed);
        assert!(!core.hotkey_registered);
        core.faults.register_recovery.set(false);
        fs::remove_dir(&settings).unwrap();
        core.apply(update.clone()).unwrap();
        assert!(core.hotkey_registered);

        let mut same_port = update.clone();
        same_port.target = free_address();
        core.faults.runtime.set(true);
        let error = core.apply(same_port.clone()).unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::Rejected);
        assert!(core.runtime.is_some());
        core.faults.runtime_recovery.set(true);
        let error = core.apply(same_port).unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::RecoveryFailed);
        assert!(core.runtime.is_none());
        assert!(!core.can_toggle());
        assert_eq!(core.status().muted, None);
        core.faults.runtime.set(false);
        core.faults.runtime_recovery.set(false);
        core.apply(update).unwrap();
        assert!(core.runtime.is_some());
        assert!(!dir.join("settings.pending.toml").exists());
        drop(core);
        fs::remove_file(settings).unwrap();
        fs::remove_file(dir.join("settings.toml.bak")).unwrap();
        fs::remove_file(dir.join("toggle-count.txt")).ok();
        fs::remove_dir(dir).unwrap();
    }

    #[test]
    fn persistence_reports_registry_update_and_rollback_failures() {
        let root = std::env::temp_dir().join(format!("alcmute-persist-{}", std::process::id()));
        fs::create_dir_all(root.join("settings.toml")).unwrap();
        let options = Options {
            data_dir: root.clone(),
            benchmark: true,
        };
        let temp = root.join("settings.pending.toml");
        fs::write(&temp, "").unwrap();
        let called = std::cell::Cell::new(false);
        let error = commit_settings_with(
            &options,
            &Config::default(),
            &temp,
            || Err("Injected registry write failure".into()),
            || {
                called.set(true);
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::Rejected);
        assert!(!called.get());
        let error = commit_settings_with(
            &options,
            &Config::default(),
            &temp,
            || Ok(()),
            || Err("Injected registry recovery failure".into()),
        )
        .unwrap_err();
        assert_eq!(error.kind, ApplyErrorKind::RecoveryFailed);
        assert!(error.to_string().contains("startup rollback failed"));
        fs::remove_file(temp).unwrap();
        fs::remove_dir(root.join("settings.toml")).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn queued_voice_pulses_keep_a_release_interval() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(std::time::Duration::from_secs(2)))
            .unwrap();
        let target = receiver.local_addr().unwrap().to_string();
        let worker = thread::spawn(move || {
            let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
            voice_pulse(&sender, &target).unwrap();
            voice_pulse(&sender, &target).unwrap();
        });
        let mut samples = Vec::new();
        for expected in [1, 0, 1, 0] {
            let mut bytes = [0; 256];
            let (len, _) = receiver.recv_from(&mut bytes).unwrap();
            let (_, packet) = rosc::decoder::decode_udp(&bytes[..len]).unwrap();
            let OscPacket::Message(message) = packet else {
                panic!()
            };
            assert_eq!(message.addr, "/input/Voice");
            assert_eq!(message.args, vec![OscType::Int(expected)]);
            samples.push(std::time::Instant::now());
        }
        assert!(
            samples[2].duration_since(samples[1]) >= std::time::Duration::from_millis(25),
            "release must survive a VRChat frame before the next press"
        );
        worker.join().unwrap();
    }
    #[test]
    fn saved_settings_contain_only_overrides_and_missing_fields_use_defaults() {
        assert_eq!(toml::to_string_pretty(&Config::default()).unwrap(), "");
        let custom = Config {
            listen: "127.0.0.1:9101".into(),
            ..Default::default()
        };
        let saved = toml::to_string_pretty(&custom).unwrap();
        assert!(saved.contains("listen"));
        assert!(
            !saved.contains("target")
                && !saved.contains("shortcut")
                && !saved.contains("autostart")
        );
        assert_eq!(toml::from_str::<Config>(&saved).unwrap(), custom);
        assert_eq!(toml::from_str::<Config>("").unwrap(), Config::default());
    }
    #[test]
    fn double_click_action_round_trips_and_old_settings_default_to_toggle() {
        let legacy: Config = toml::from_str("shortcut = 'Ctrl+Alt+KeyM'").unwrap();
        assert!(!legacy.tray_double_click_settings);
        let config = Config {
            tray_double_click_settings: true,
            ..Config::default()
        };
        let saved = toml::to_string_pretty(&config).unwrap();
        assert!(saved.contains("tray_double_click_settings = true"));
        assert_eq!(toml::from_str::<Config>(&saved).unwrap(), config);
        assert!(!toml::to_string(&Config::default())
            .unwrap()
            .contains("tray_double_click_settings"));
    }
    #[test]
    fn startup_window_defaults_to_visible_and_persists_disabled() {
        assert!(
            toml::from_str::<Config>("")
                .unwrap()
                .show_settings_on_startup
        );
        let config = Config {
            show_settings_on_startup: false,
            ..Config::default()
        };
        let saved = toml::to_string_pretty(&config).unwrap();
        assert!(saved.contains("show_settings_on_startup = false"));
        assert!(
            !toml::from_str::<Config>(&saved)
                .unwrap()
                .show_settings_on_startup
        );
        assert_eq!(toml::to_string_pretty(&Config::default()).unwrap(), "");
    }
    #[test]
    fn refresh_is_dispatched_off_the_caller_thread_and_receiver_stops() {
        let config = Config {
            listen: "127.0.0.1:0".into(),
            ..Config::default()
        };
        let status = Arc::new(Mutex::new(Status::default()));
        let (events, received) = mpsc::channel();
        let notify: Notify = Arc::new(move |event| {
            let _ = events.send((event, thread::current().id()));
        });
        let options = Options {
            data_dir: std::env::temp_dir(),
            benchmark: false,
        };
        let worker = runtime(&config, status.clone(), notify, &options).unwrap();
        worker.tx.send(Command::Refresh).unwrap();
        let (_, callback_thread) = received
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_ne!(
            callback_thread,
            thread::current().id(),
            "UI notification must not reenter the caller"
        );
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        sender.send_to(b"invalid OSC", worker.address).unwrap();
        sender
            .send_to(
                &encode("/avatar/parameters/MuteSelf", vec![OscType::Bool(true)]).unwrap(),
                worker.address,
            )
            .unwrap();
        received
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(status.lock().unwrap().muted, Some(true));
        let start = std::time::Instant::now();
        drop(worker);
        assert!(start.elapsed() < std::time::Duration::from_secs(2));
    }
    #[test]
    fn validate_settings_and_round_trip() {
        let config = Config::default();
        config.validate().unwrap();
        assert_eq!(
            toml::from_str::<Config>(&toml::to_string(&config).unwrap()).unwrap(),
            config
        );
        let mut invalid = config.clone();
        invalid.listen = "0.0.0.0:9001".into();
        assert!(invalid.validate().is_err());
        invalid = config.clone();
        invalid.shortcut = "not-a-key".into();
        assert!(invalid.validate().is_err());
        invalid = config.clone();
        invalid.target = config.listen;
        assert!(invalid.validate().is_err());
    }
    #[test]
    fn osc_state_requires_boolean_and_handles_bundles() {
        let bytes = encode("/avatar/parameters/MuteSelf", vec![OscType::Bool(true)]).unwrap();
        let (_, packet) = rosc::decoder::decode_udp(&bytes).unwrap();
        assert_eq!(mute_value(&packet), Some(true));
        let bundle = OscPacket::Bundle(rosc::OscBundle {
            timetag: (0, 1).into(),
            content: vec![packet],
        });
        assert_eq!(mute_value(&bundle), Some(true));
        let (_, wrong) = rosc::decoder::decode_udp(
            &encode("/avatar/parameters/MuteSelf", vec![OscType::Int(1)]).unwrap(),
        )
        .unwrap();
        assert_eq!(mute_value(&wrong), None);
    }
    #[test]
    fn voice_always_sends_press_and_release() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(std::time::Duration::from_secs(1)))
            .unwrap();
        let sender = UdpSocket::bind("127.0.0.1:0").unwrap();
        voice_pulse(&sender, &receiver.local_addr().unwrap().to_string()).unwrap();
        let mut buf = [0; 1024];
        for expected in [1, 0] {
            let count = receiver.recv(&mut buf).unwrap();
            let (_, packet) = rosc::decoder::decode_udp(&buf[..count]).unwrap();
            match packet {
                OscPacket::Message(message) => {
                    assert_eq!(message.addr, "/input/Voice");
                    assert_eq!(message.args, vec![OscType::Int(expected)]);
                }
                _ => panic!("Expected a message"),
            }
        }
    }
}
