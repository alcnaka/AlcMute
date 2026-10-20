use global_hotkey::{GlobalHotKeyEvent, HotKeyState};
mod shortcut;
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
    #[serde(skip_serializing_if = "is_false")]
    pub autostart: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub tray_double_click_settings: bool,
    #[serde(skip_serializing_if = "is_true")]
    pub show_settings_on_startup: bool,
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
        }
    }
}
impl Config {
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
impl Options {
    pub fn from_args() -> Result<Self> {
        let mut options = Self {
            data_dir: std::env::var_os("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join("VRChatMuteController"),
            benchmark: false,
        };
        let mut args = std::env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--test-mode" => options.benchmark = true,
                "--data-dir" => {
                    options.data_dir =
                        PathBuf::from(args.next().ok_or("--data-dir requires a path")?)
                }
                _ => return Err(format!("Unknown argument: {arg}")),
            }
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
        let _ = fs::create_dir_all(&self.data_dir);
        let _ = fs::write(self.data_dir.join("error.log"), error);
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
        OscPacket::Bundle(bundle) => bundle.content.iter().filter_map(mute_value).last(),
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

enum Command {
    Toggle,
    Refresh,
    Stop,
}
struct Runtime {
    tx: mpsc::Sender<Command>,
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
    let (tx, rx) = mpsc::channel();
    let stop = Arc::new(AtomicBool::new(false));
    let target = config.target.clone();
    let command_status = status.clone();
    let command_notify = notify.clone();
    let command_dir = options.data_dir.clone();
    let commands = thread::spawn(move || {
        let mut toggle_count = 0u64;
        while let Ok(command) = rx.recv() {
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
    runtime: Option<Runtime>,
    status: Arc<Mutex<Status>>,
    notify: Notify,
}
impl Controller {
    pub fn new(options: Options, notify: Notify) -> Result<Self> {
        fs::create_dir_all(&options.data_dir).map_err(|e| e.to_string())?;
        let path = options.data_dir.join("settings.toml");
        #[allow(unused_mut)]
        let mut config: Config = if path.exists() {
            toml::from_str(&fs::read_to_string(path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?
        } else {
            Config::default()
        };
        #[cfg(feature = "windows-integration")]
        {
            config.autostart = windows::startup_enabled(&options)?;
        }
        config.validate()?;
        let manager = GlobalHotKeyManager::new().map_err(|e| e.to_string())?;
        let hotkey: HotKey = config.shortcut.parse().map_err(|e| format!("{e}"))?;
        manager
            .register(hotkey)
            .map_err(|e| format!("Shortcut is unavailable: {e}"))?;
        let status = Arc::new(Mutex::new(Status::default()));
        let runtime = runtime(&config, status.clone(), notify.clone(), &options)?;
        install_handler(&runtime, hotkey);
        Ok(Self {
            config,
            options,
            manager,
            hotkey,
            hotkey_registered: true,
            runtime: Some(runtime),
            status,
            notify,
        })
    }
    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }
    pub fn toggle(&self) {
        if let Some(runtime) = &self.runtime {
            let _ = runtime.tx.send(Command::Toggle);
        }
    }
    pub fn apply(&mut self, config: Config) -> Result<()> {
        config.validate()?;
        let hotkey: HotKey = config.shortcut.parse().map_err(|e| format!("{e}"))?;
        let changed = hotkey != self.hotkey;
        let needs_registration = changed || !self.hotkey_registered;
        if needs_registration {
            self.manager
                .register(hotkey)
                .map_err(|e| format!("Shortcut is unavailable: {e}"))?;
        }
        // Prepare the file before disrupting the existing listener.
        let temp = self.options.data_dir.join("settings.pending.toml");
        let serialized = toml::to_string_pretty(&config).map_err(|e| e.to_string())?;
        if let Err(e) = fs::write(&temp, serialized) {
            if needs_registration {
                let _ = self.manager.unregister(hotkey);
            }
            return Err(e.to_string());
        }
        self.runtime.take();
        let candidate = runtime(
            &config,
            self.status.clone(),
            self.notify.clone(),
            &self.options,
        );
        let candidate = match candidate {
            Ok(value) => value,
            Err(error) => {
                if needs_registration {
                    let _ = self.manager.unregister(hotkey);
                }
                self.runtime = runtime(
                    &self.config,
                    self.status.clone(),
                    self.notify.clone(),
                    &self.options,
                )
                .ok();
                if let Some(old) = &self.runtime {
                    install_handler(old, self.hotkey);
                }
                return Err(format!("{error}; previous settings retained"));
            }
        };
        let commit = commit_settings(&self.options, &config, &temp);
        if let Err(e) = commit {
            drop(candidate);
            if needs_registration {
                let _ = self.manager.unregister(hotkey);
            }
            self.runtime = runtime(
                &self.config,
                self.status.clone(),
                self.notify.clone(),
                &self.options,
            )
            .ok();
            if let Some(old) = &self.runtime {
                install_handler(old, self.hotkey);
            }
            return Err(format!("Cannot save settings: {e}"));
        }
        if changed && self.hotkey_registered {
            let _ = self.manager.unregister(self.hotkey);
        }
        install_handler(&candidate, hotkey);
        self.runtime = Some(candidate);
        self.config = config;
        self.hotkey = hotkey;
        self.hotkey_registered = true;
        let _ = fs::remove_file(&temp);
        *self.status.lock().unwrap() = Status::default();
        // NWG NoticeSender uses SendNotifyMessageW, which is synchronous on the
        // UI thread. Always notify from a worker to avoid re-entering apply().
        if let Some(runtime) = &self.runtime {
            let _ = runtime.tx.send(Command::Refresh);
        }
        Ok(())
    }
    pub fn reset_to_defaults(&mut self) -> Result<()> {
        self.apply(Config::default())
    }
    pub fn begin_shortcut_capture(&mut self) -> Result<()> {
        self.manager.begin_capture()?;
        if self.hotkey_registered {
            self.manager
                .unregister(self.hotkey)
                .map_err(|e| e.to_string())?;
            self.hotkey_registered = false;
        }
        if let Some(route) = HOTKEY_ROUTE.get() {
            route.lock().unwrap().take();
        }
        Ok(())
    }
    pub fn end_shortcut_capture(&mut self) -> Result<()> {
        self.manager.end_capture();
        if !self.hotkey_registered {
            self.manager
                .register(self.hotkey)
                .map_err(|e| format!("ショートカットの再登録に失敗しました: {e}"))?;
            self.hotkey_registered = true;
        }
        if let Some(runtime) = &self.runtime {
            install_handler(runtime, self.hotkey);
        }
        Ok(())
    }
}
fn commit_settings(options: &Options, config: &Config, temp: &std::path::Path) -> Result<()> {
    #[cfg(feature = "windows-integration")]
    let previous = windows::startup_snapshot(options)?;
    #[cfg(feature = "windows-integration")]
    windows::set_startup(options, config.autostart)?;
    let path = options.data_dir.join("settings.toml");
    let result = if config == &Config::default() {
        match fs::remove_file(&path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            result => result,
        }
    } else {
        fs::rename(temp, path)
    };
    if let Err(error) = result {
        #[cfg(feature = "windows-integration")]
        windows::restore_startup(options, previous.as_deref())
            .map_err(|rollback| format!("{error}; startup rollback failed: {rollback}"))?;
        return Err(error.to_string());
    }
    let _ = fs::remove_file(temp);
    Ok(())
}
// global-hotkey's handler is a OnceCell: install once, route through a mutable slot.
static HOTKEY_ROUTE: OnceLock<Mutex<Option<(u32, mpsc::Sender<Command>)>>> = OnceLock::new();
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
