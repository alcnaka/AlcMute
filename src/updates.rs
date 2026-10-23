use crate::{update_install, Settings, Tray};
use alcmute_core::{
    updater::{self, Preferences, Release},
    Options,
};
use slint::ComponentHandle;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

struct State {
    preferences: Preferences,
    message: String,
    latest: Option<Release>,
    busy: bool,
    last: Option<Instant>,
}
#[derive(Clone)]
pub struct Updates {
    state: Arc<Mutex<State>>,
    window: Arc<Mutex<Option<slint::Weak<Settings>>>>,
    tray: slint::Weak<Tray>,
    options: Options,
}
impl Updates {
    pub fn new(options: Options, tray: slint::Weak<Tray>) -> Self {
        let (preferences, message) = match Preferences::load(&options) {
            Ok(p) => (p, String::new()),
            Err(e) => (Preferences { auto_check: false }, e),
        };
        Self {
            state: Arc::new(Mutex::new(State {
                preferences,
                message,
                latest: None,
                busy: false,
                last: None,
            })),
            window: Arc::new(Mutex::new(None)),
            tray,
            options,
        }
    }
    pub fn start(&self) {
        if !self.options.benchmark && self.state.lock().unwrap().preferences.auto_check {
            self.check_inner(true);
        }
    }
    pub fn attach(&self, ui: &Settings) {
        ui.set_app_version(
            format!(
                "{} / {}",
                env!("CARGO_PKG_VERSION"),
                if cfg!(feature = "installer") {
                    "Installer"
                } else {
                    "Portable"
                }
            )
            .into(),
        );
        *self.window.lock().unwrap() = Some(ui.as_weak());
        self.restore_editor(ui);
        let service = self.clone();
        ui.on_check_updates(move || service.check());
        let service = self.clone();
        ui.on_install_update(move || service.install());
        let service = self.clone();
        ui.on_update_preferences(move |enabled| match service.save_preferences(enabled) {
            Ok(()) => {
                if let Some(ui) = service
                    .window
                    .lock()
                    .unwrap()
                    .as_ref()
                    .and_then(slint::Weak::upgrade)
                {
                    ui.set_auto_check_updates(enabled);
                }
                true
            }
            Err(error) => {
                if let Some(ui) = service
                    .window
                    .lock()
                    .unwrap()
                    .as_ref()
                    .and_then(slint::Weak::upgrade)
                {
                    ui.set_save_ok(false);
                    ui.set_message(error.into());
                }
                false
            }
        });
        let service = self.clone();
        ui.on_release_page(move || {
            if let Err(error) = std::process::Command::new("explorer.exe")
                .arg(updater::RELEASES)
                .spawn()
            {
                service.state.lock().unwrap().message =
                    format!("Releaseページを開けませんでした: {error}");
                service.refresh();
            }
        });
        self.refresh();
    }
    fn refresh(&self) {
        let state = self.state.lock().unwrap();
        if let Some(tray) = self.tray.upgrade() {
            tray.set_update_available(state.latest.is_some());
        }
        let weak = self.window.lock().unwrap();
        if let Some(ui) = weak.as_ref().and_then(slint::Weak::upgrade) {
            ui.set_update_status(state.message.clone().into());
            ui.set_update_busy(state.busy);
            ui.set_update_available(state.latest.is_some());
            ui.set_update_installable(alcmute_core::windows::startup_available(&self.options));
        }
    }
    pub fn restore_editor(&self, ui: &Settings) {
        let enabled = self.state.lock().unwrap().preferences.auto_check;
        ui.set_auto_check_updates(enabled);
        ui.set_saved_auto_check_updates(enabled);
    }
    pub fn save_preferences(&self, enabled: bool) -> Result<(), String> {
        let preferences = Preferences {
            auto_check: enabled,
        };
        preferences.save(&self.options)?;
        self.state.lock().unwrap().preferences = preferences;
        if let Some(ui) = self
            .window
            .lock()
            .unwrap()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            ui.set_saved_auto_check_updates(enabled);
        }
        Ok(())
    }
    pub fn check(&self) {
        self.check_inner(false);
    }
    fn check_inner(&self, automatic: bool) {
        let mut state = self.state.lock().unwrap();
        if state.busy {
            return;
        }
        if automatic
            && state
                .last
                .is_some_and(|last| last.elapsed() < Duration::from_secs(60))
        {
            return;
        }
        state.last = Some(Instant::now());
        state.busy = true;
        state.message = "更新を確認しています…".into();
        drop(state);
        self.refresh();
        let service = self.clone();
        std::thread::spawn(move || {
            let result = updater::fetch_latest();
            let mut state = service.state.lock().unwrap();
            state.busy = false;
            match result.and_then(|release| {
                release
                    .map(|release| {
                        release
                            .newer_than(env!("CARGO_PKG_VERSION"))
                            .map(|newer| (release, newer))
                    })
                    .transpose()
            }) {
                Ok(Some((release, true))) => {
                    state.message = format!("新しいバージョン {} があります。", release.tag_name);
                    state.latest = Some(release);
                }
                Ok(_) => {
                    state.message = "現在のバージョンは最新です。".into();
                    state.latest = None;
                }
                Err(error) => {
                    service.options.log(&error);
                    state.message = format!("更新確認に失敗しました: {error}");
                }
            }
            drop(state);
            let _ = slint::invoke_from_event_loop(move || service.refresh());
        });
    }
    fn install(&self) {
        if alcmute_core::windows::installation_directory().is_err() {
            return;
        }
        let state = self.state.lock().unwrap();
        if state.busy {
            return;
        }
        let Some(release) = state.latest.clone() else {
            return;
        };
        drop(state);
        if !alcmute_core::windows::confirm("AlcMuteの更新",&format!("{}をダウンロードし、AlcMuteを終了して更新しますか？\n未保存の設定変更は保存されません。",release.tag_name)){return;}
        let mut state = self.state.lock().unwrap();
        state.busy = true;
        state.message = "インストーラーをダウンロード・検証しています…".into();
        drop(state);
        self.refresh();
        let service = self.clone();
        std::thread::spawn(move || {
            let result = update_install::directory().and_then(|dir| updater::stage(&release, &dir));
            let _ = slint::invoke_from_event_loop(move || {
                let result = result.and_then(|setup| {
                    update_install::launch(
                        &setup,
                        &release.version()?.to_string(),
                        &service.options,
                    )
                });
                match result {
                    Ok(()) => {
                        let _ = slint::quit_event_loop();
                    }
                    Err(error) => {
                        service.options.log(&error);
                        let mut state = service.state.lock().unwrap();
                        state.busy = false;
                        state.message = format!("更新に失敗しました: {error}");
                        drop(state);
                        service.refresh();
                    }
                }
            });
        });
    }
}
