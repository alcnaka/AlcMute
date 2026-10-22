#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use alcmute_core::{Config, Connection, Controller, Event, Options};
use slint::winit_030::WinitWindowAccessor;
use slint::ComponentHandle;
mod about;
mod shortcut;
mod update_install;
mod updates;
use std::{cell::RefCell, rc::Rc, sync::Arc};
slint::slint! {
    import { VerticalBox, HorizontalBox, LineEdit, Button, CheckBox, ScrollView } from "std-widgets.slint";
    component ResetButton inherits FocusScope {
        in property <string> setting-name;
        enabled: true;
        callback clicked();
        accessible-role: button;
        accessible-label: root.setting-name + "をデフォルトに戻す";
        focus-on-tab-navigation: root.enabled;
        width: 18px; height: 18px;
        vertical-stretch: 0;
        key-pressed(event) => {
            if root.enabled && (event.text == " " || event.text == Key.Return) {
                root.clicked(); return accept;
            }
            return reject;
        }
        Image {
            source: @image-url("../assets/reset.svg");
            opacity: !root.enabled ? 0.25 : (touch.has-hover || root.has-focus ? 0.95 : 0.5);
        }
        touch := TouchArea { enabled: root.enabled; clicked => { root.focus(); root.clicked(); } }
    }
    component SectionTitle inherits Text {
        font-size: 19px; font-weight: 700; vertical-stretch: 0;
    }
    component FieldLabel inherits Text {
        font-size: 16px; font-weight: 600; vertical-stretch: 0;
    }
    component Hint inherits Text {
        font-size: 14px; opacity: 0.75; wrap: word-wrap; vertical-stretch: 0;
    }
    component RadioChoice inherits FocusScope {
        in property <string> text;
        in property <bool> checked;
        callback selected();
        callback navigate();
        min-height: 36px; vertical-stretch: 0;
        focus-on-tab-navigation: root.enabled;
        accessible-role: radio-button;
        accessible-label: root.text;
        accessible-checkable: true;
        accessible-checked: root.checked;
        accessible-enabled: root.enabled;
        accessible-action-default => { if root.enabled {root.selected();} }
        key-pressed(event) => {
            if event.text == Key.Space || event.text == Key.Return {root.selected(); return accept;}
            if event.text == Key.UpArrow || event.text == Key.DownArrow || event.text == Key.LeftArrow || event.text == Key.RightArrow {root.navigate(); return accept;}
            return reject;
        }
        opacity: root.enabled ? 1 : 0.45;
            Rectangle {
                x: 0px; y: (root.height - self.height) / 2;
                width: 20px; height: 20px; border-radius: 10px;
                border-width: 2px;
                border-color: root.checked || root.has-focus ? #438de8 : #888;
                Rectangle {
                    x: 5px; y: 5px; width: 10px; height: 10px; border-radius: 5px;
                    background: #438de8; visible: root.checked;
                }
            }
            Text {
                x: 30px; y: 0px; width: root.width - 30px; height: root.height;
                text: root.text; font-size: 15px; vertical-alignment: center;
                font-weight: root.has-focus ? 600 : 400;
            }
        TouchArea { enabled: root.enabled; clicked => {root.focus(); root.selected();} }
    }
    export component Settings inherits Window {
        title: "AlcMute";
        icon: @image-url("../assets/app.png");
        preferred-width: 640px; preferred-height: 800px;
        min-width: 480px; min-height: 420px;
        default-font-size: 15px;
        in-out property <string> target;
        in-out property <string> listen;
        in-out property <string> shortcut;
        in-out property <string> shortcut-label;
        in-out property <bool> autostart;
        in property <bool> autostart-available;
        in-out property <bool> show-settings-on-startup: true;
        in-out property <int> tray-double-click-action;
        out property <bool> double-click-toggle-selected: toggle-choice.checked;
        out property <bool> double-click-settings-selected: settings-choice.checked;
        in-out property <bool> capturing;
        in-out property <string> capture-hint;
        in property <string> saved-target;
        in property <string> saved-listen;
        in property <string> saved-shortcut;
        in property <bool> saved-autostart;
        in property <bool> saved-show-settings-on-startup;
        in property <int> saved-tray-double-click-action;
        out property <bool> dirty: root.target != root.saved-target
            || root.listen != root.saved-listen || root.shortcut != root.saved-shortcut
            || root.autostart != root.saved-autostart
            || root.show-settings-on-startup != root.saved-show-settings-on-startup
            || root.tray-double-click-action != root.saved-tray-double-click-action;
        in property <string> config-path;
        in property <bool> can-toggle;
        in property <bool> save-ok;
        in property <string> state;
        in property <string> message;
        in-out property <bool> auto-check-updates;
        in property <string> update-status;
        in property <bool> update-busy;
        in property <bool> update-available;
        in property <bool> update-installable;
        in property <string> app-version;
        callback check-updates();
        callback install-update();
        callback release-page();
        callback update-preferences(bool);
        callback save(string, string, string, bool);
        callback toggle();
        callback dismiss();
        callback about();
        callback quit-confirmed();
        callback reset-field(int);
        callback reset-all();
        callback begin-capture();
        callback capture-key();
        callback cancel-capture();
        key-scope := FocusScope {
            key-pressed(event) => {
                if root.capturing {
                    if event.text == Key.Escape { root.cancel-capture(); }
                    else { root.capture-key(); }
                    return accept;
                }
                return reject;
            }
            VerticalBox {
                padding: 20px; spacing: 14px;
                Text { text: "AlcMute"; font-size: 26px; font-weight: 700; vertical-stretch: 0; }
                Hint { text: "あるみゅーと / VRC Background Mute　" + root.app-version; }
                HorizontalBox {
                    Text { text: root.state; font-size: 16px; wrap: word-wrap; }
                    Button { text: "ミュート切り替え"; enabled: root.can-toggle && !root.capturing; clicked => {root.toggle();} }
                }
                Rectangle { height: 1px; background: #888; opacity: 0.25; }
                ScrollView {
                    vertical-stretch: 1;
                    VerticalBox {
                        padding: 4px; spacing: 12px; alignment: start;
                        SectionTitle { text: "操作"; }
                        HorizontalBox {
                            FieldLabel { text: "ミュート切り替えのショートカット"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "ショートカット"; enabled: !root.capturing; clicked => {root.reset-field(2);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        Button {
                            text: root.capturing ? "記録をキャンセル（Esc）" : root.shortcut-label + "　 キーを変更";
                            clicked => {
                                if root.capturing {root.cancel-capture();}
                                else {root.begin-capture(); if root.capturing {key-scope.focus();}}
                            }
                        }
                        Hint { text: root.capturing ? root.capture-hint : "K＋Lなど通常キー同士も登録できます。すべてのキーを離すと確定します。"; }
                        HorizontalBox {
                            FieldLabel { text: "トレイアイコンのダブルクリック"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "ダブルクリック動作"; enabled: !root.capturing; clicked => {root.reset-field(4);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        toggle-choice := RadioChoice {
                            text: "ミュート切り替え（デフォルト）";
                            checked: root.tray-double-click-action == 0;
                            enabled: !root.capturing;
                            selected => {root.tray-double-click-action = 0;}
                            navigate => {root.tray-double-click-action = 1; settings-choice.focus();}
                        }
                        settings-choice := RadioChoice {
                            text: "設定ウィンドウを開く";
                            checked: root.tray-double-click-action == 1;
                            enabled: !root.capturing;
                            selected => {root.tray-double-click-action = 1;}
                            navigate => {root.tray-double-click-action = 0; toggle-choice.focus();}
                        }
                        Rectangle { height: 1px; background: #888; opacity: 0.25; }
                        SectionTitle { text: "起動"; }
                        HorizontalBox {
                            FieldLabel { text: "Windowsサインイン時"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "自動起動"; enabled: !root.capturing && root.autostart-available; clicked => {root.reset-field(3);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        CheckBox { text: "Windowsサインイン時に起動"; checked <=> root.autostart; enabled: !root.capturing && root.autostart-available; }
                        if !root.autostart-available: Hint { text: "インストーラー版でのみ利用できます。"; }
                        HorizontalBox {
                            FieldLabel { text: "起動時の画面"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "起動時の表示"; enabled: !root.capturing; clicked => {root.reset-field(5);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        CheckBox { text: "手動起動時に設定ウィンドウを表示する"; checked <=> root.show-settings-on-startup; enabled: !root.capturing; }
                        Hint { text: "Windowsサインイン時と更新後は画面を表示せず常駐します。"; }
                        Rectangle { height: 1px; background: #888; opacity: 0.25; }
                        SectionTitle { text: "OSC接続"; }
                        Hint { text: "VRChatと通信するアドレスです。通常は初期値のままで使用できます。"; }
                        HorizontalBox {
                            FieldLabel { text: "送信先（IPアドレス：ポート）"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "OSC送信先"; enabled: !root.capturing; clicked => {root.reset-field(0);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        LineEdit { text <=> root.target; enabled: !root.capturing; }
                        HorizontalBox {
                            FieldLabel { text: "受信先（IPアドレス：ポート）"; horizontal-stretch: 0; }
                            ResetButton { setting-name: "OSC受信先"; enabled: !root.capturing; clicked => {root.reset-field(1);} }
                            Rectangle { horizontal-stretch: 1; }
                        }
                        LineEdit { text <=> root.listen; enabled: !root.capturing; }
                        Rectangle { height: 1px; background: #888; opacity: 0.25; }
                        SectionTitle { text: "管理"; }
                        CheckBox { text: "起動時にアップデートを確認"; checked <=> root.auto-check-updates; enabled: !root.update-busy && !root.capturing; toggled => {root.update-preferences(self.checked);} }
                        Hint { text: "更新確認の設定はすぐに保存されます。"; }
                        Button { text: "アップデートを確認"; enabled: !root.update-busy && !root.capturing; clicked => {root.check-updates();} }
                        if root.update-status != "": Hint { text: root.update-status; }
                        if root.update-available && root.update-installable: Button { text: "ダウンロードして更新"; enabled: !root.update-busy && !root.capturing; clicked => {root.install-update();} }
                        Button { text: "GitHub Releasesを開く"; clicked => {root.release-page();} }
                        Button { text: "AlcMuteについて"; enabled: !root.capturing; clicked => {root.about();} }
                        Hint { text: "設定ファイル：\n" + root.config-path; }
                        Button { text: "すべての設定を初期化"; enabled: !root.capturing; clicked => {root.reset-all();} }
                        Hint { text: "確認後に保存済み設定を削除し、自動起動も無効にします。"; }
                        Button { text: "ツールを終了"; clicked => {root.cancel-capture(); root.quit-confirmed();} }
                    }
                }
                Rectangle { height: 1px; background: #888; opacity: 0.25; }
                VerticalBox {
                    padding: 0px; spacing: 8px; vertical-stretch: 0;
                    if root.dirty: Text { text: "保存されていない設定があります"; color: #d58b28; font-size: 15px; wrap: word-wrap; }
                    if root.message != "": Text { text: root.message; wrap: word-wrap; color: root.save-ok ? #16814b : #b04030; font-size: 14px; }
                    HorizontalBox {
                        Hint { text: "変更は「保存」で反映されます。"; }
                        Button { text: "保存"; enabled: root.dirty && !root.capturing; primary: root.dirty && !root.capturing; clicked => {root.save(root.target, root.listen, root.shortcut, root.autostart);} }
                        Button { text: "閉じる"; clicked => {root.cancel-capture(); root.dismiss();} }
                    }
                }
            }
        }
    }
    export component Tray inherits SystemTrayIcon {
        in property <bool> update-available;
        in property <string> status-text: "AlcMute - Unknown";
        in property <int> mute-state;
        in property <bool> can-toggle;
        icon: root.mute-state == 1 ? @image-url("../assets/mic-muted.svg") : root.mute-state == 2 ? @image-url("../assets/mic-unmuted.svg") : @image-url("../assets/mic-unknown.svg");
        tooltip: root.status-text + (root.update-available ? " / 更新があります" : "");
        callback toggle(); callback settings(); callback about(); callback exit-requested(); callback updates();
        callback icon-clicked();
        clicked => { root.icon-clicked(); }
        Menu {
            MenuItem { title: "ミュート切り替え"; enabled: root.can-toggle; activated => { root.toggle(); } }
            MenuItem { title: "設定"; activated => { root.settings(); } }
            MenuItem { title: root.update-available ? "更新があります" : "アップデートを確認"; activated => {root.updates();} }
            MenuItem { title: "AlcMuteについて"; activated => { root.about(); } }
            MenuItem { title: "終了"; activated => { root.exit-requested(); } }
        }
    }
}
fn main() -> std::process::ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // Installer preflight: no settings, OSC, startup registry, or UI initialization.
    if args.as_slice() == ["--check-install-ready"] {
        return match alcmute_core::windows::SingleInstance::acquire() {
            Ok(Some(_instance)) => std::process::ExitCode::SUCCESS,
            Ok(None) => std::process::ExitCode::from(2),
            Err(_) => std::process::ExitCode::FAILURE,
        };
    }
    if args.as_slice() == ["--verify-licenses"] {
        let resource_ok = std::env::current_exe()
            .ok()
            .and_then(|path| alcmute_core::windows::executable_version(&path).ok())
            .is_some_and(|version| version == env!("CARGO_PKG_VERSION"));
        return if about::notices().is_ok() && resource_ok {
            std::process::ExitCode::SUCCESS
        } else {
            std::process::ExitCode::FAILURE
        };
    }
    if args.first().is_some_and(|arg| arg == "--update-helper") {
        return match update_install::helper(&args) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                Options::default().log(&error);
                std::process::ExitCode::FAILURE
            }
        };
    }
    let options = match Options::from_args() {
        Ok(options) => options,
        Err(error) => {
            report_startup_error(&Options::default(), &error);
            return std::process::ExitCode::FAILURE;
        }
    };
    options.install_panic_log();
    let background = std::env::args().any(|arg| arg == "--background");
    let maintenance = if std::env::args().any(|arg| arg == "--install-migrate") {
        Some(true)
    } else if std::env::args().any(|arg| arg == "--uninstall-cleanup") {
        Some(false)
    } else {
        None
    };
    if let Some(install) = maintenance {
        let _maintenance_instance = match alcmute_core::windows::SingleInstance::acquire() {
            Ok(Some(instance)) => instance,
            _ => {
                options.log("インストール・アンインストールの前に、起動中のAlcMute（旧Portable版を含む）を終了してください。");
                return std::process::ExitCode::FAILURE;
            }
        };
        let result = if install {
            alcmute_core::windows::migrate_startup(&options)
        } else {
            alcmute_core::windows::uninstall_cleanup(&options)
        };
        return match result {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(error) => {
                options.log(&error);
                std::process::ExitCode::FAILURE
            }
        };
    }
    let _instance = match alcmute_core::windows::SingleInstance::acquire() {
        Ok(Some(instance)) => instance,
        Ok(None) => {
            if !background {
                if let Err(error) = alcmute_core::windows::ipc::request_settings() {
                    report_error(&options, "起動済みのAlcMuteに接続できません", &error);
                    return std::process::ExitCode::FAILURE;
                }
            }
            return std::process::ExitCode::SUCCESS;
        }
        Err(error) => {
            report_startup_error(&options, &error);
            return std::process::ExitCode::FAILURE;
        }
    };
    let _installer_guard = match alcmute_core::windows::SingleInstance::installer_guard() {
        Ok(Some(guard)) => guard,
        _ => {
            report_startup_error(&options, "インストール用の排他制御を取得できませんでした");
            return std::process::ExitCode::FAILURE;
        }
    };
    if alcmute_core::settings_store::recovery_available(&options) && alcmute_core::windows::confirm(
        "設定ファイルの復旧",
        &format!("{} が破損しています。正常なsettings.toml.bakから復旧しますか？\n破損したファイルは別名で保全します。",options.data_dir.join("settings.toml").display())
    ) {
        if let Err(error)=alcmute_core::settings_store::recover(&options) {report_startup_error(&options,&error);return std::process::ExitCode::FAILURE;}
    }
    if let Err(error) = run(options.clone()) {
        report_startup_error(&options, &error);
        return std::process::ExitCode::FAILURE;
    }
    std::process::ExitCode::SUCCESS
}
fn report_startup_error(options: &Options, error: &str) {
    report_error(options, "AlcMuteの起動に失敗しました", error);
}
fn report_error(options: &Options, title: &str, error: &str) {
    let log = match options.write_log(error) {
        Ok(path) => format!("ログ：{}", path.display()),
        Err(reason) => format!(
            "ログを保存できませんでした：{reason}\n保存先：{}",
            options.data_dir.join("error.log").display()
        ),
    };
    alcmute_core::windows::show_error(title, &format!("{error}\n\n{log}\n\n設定ファイル・OSC受信ポート・ショートカットを確認して、再起動してください。"));
}
fn run(options: Options) -> Result<(), String> {
    let tray = Tray::new().map_err(|e| e.to_string())?;
    // Lazy creation avoids paying for settings controls until the first use.
    let settings: Rc<RefCell<Option<Settings>>> = Rc::new(RefCell::new(None));
    let updates = updates::Updates::new(options.clone(), tray.as_weak());
    let controller: Rc<RefCell<Option<Controller>>> = Rc::new(RefCell::new(None));
    let options_for_events = options.clone();
    let weak_tray = tray.as_weak();
    let controller_events = controller.clone();
    let settings_events = settings.clone();
    let show_about: Rc<dyn Fn()> = {
        let about = Rc::new(RefCell::new(None));
        let options = options.clone();
        Rc::new(move || {
            if let Err(error) = about::show(&about) {
                report_error(&options, "AlcMuteについてを表示できません", &error);
            }
        })
    };
    let show: Rc<dyn Fn()> = {
        let updates = updates.clone();
        let controller = controller.clone();
        let settings = settings.clone();
        let options = options.clone();
        let show_about = show_about.clone();
        Rc::new(move || {
            if settings.borrow().is_none() {
                let ui = match Settings::new() {
                    Ok(ui) => ui,
                    Err(e) => {
                        report_error(&options, "設定画面を表示できません", &e.to_string());
                        return;
                    }
                };
                install_editing(&ui, controller.clone(), options.clone());
                updates.attach(&ui);
                ui.set_autostart_available(alcmute_core::windows::startup_available(&options));
                let show_about = show_about.clone();
                ui.on_about(move || show_about());
                let weak = ui.as_weak();
                let controller_save = controller.clone();
                ui.on_save(move |target, listen, shortcut, autostart| {
                    if let Some(core) = controller_save.borrow_mut().as_mut() {
                        let result = core.apply(Config {
                            target: target.trim().into(),
                            listen: listen.trim().into(),
                            shortcut: shortcut.trim().into(),
                            autostart,
                            show_settings_on_startup: weak
                                .upgrade()
                                .is_none_or(|ui| ui.get_show_settings_on_startup()),
                            tray_double_click_settings: weak
                                .upgrade()
                                .is_some_and(|ui| ui.get_tray_double_click_action() == 1),
                        });
                        if let Some(ui) = weak.upgrade() {
                            if result.is_ok() {
                                fill_config(&ui, &core.config);
                            }
                            ui.set_save_ok(result.is_ok());
                            ui.set_message(match result {
                                Ok(()) => "保存しました".into(),
                                Err(error) => error.to_string().into(),
                            });
                        }
                    }
                });
                let controller_toggle = controller.clone();
                ui.on_toggle(move || {
                    if let Some(core) = controller_toggle.borrow().as_ref() {
                        core.toggle();
                    }
                });
                let weak = ui.as_weak();
                let close_options = options.clone();
                ui.on_dismiss(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.invoke_cancel_capture();
                        let _ = ui.hide();
                        close_options.marker("closed");
                    }
                });
                ui.on_quit_confirmed(|| {
                    if alcmute_core::windows::confirm_tool_exit() {
                        let _ = slint::quit_event_loop();
                    }
                });
                let weak = ui.as_weak();
                let close_options = options.clone();
                ui.window().on_close_requested(move || {
                    if let Some(ui) = weak.upgrade() {
                        ui.invoke_cancel_capture();
                        let _ = ui.hide();
                    }
                    close_options.marker("closed");
                    slint::CloseRequestResponse::KeepWindowShown
                });
                *settings.borrow_mut() = Some(ui);
            }
            if let Some(ui) = settings.borrow().as_ref() {
                // An already-open editor keeps its unsaved values and recording
                // state. Tray actions only restore/activate that same window.
                if ui.window().is_visible() {
                    activate_settings(ui);
                    return;
                }
                ui.invoke_cancel_capture();
                if let Some(core) = controller.borrow().as_ref() {
                    ui.set_target(core.config.target.clone().into());
                    ui.set_listen(core.config.listen.clone().into());
                    ui.set_shortcut(core.config.shortcut.clone().into());
                    ui.set_shortcut_label(shortcut::label(&core.config.shortcut).into());
                    ui.set_config_path(
                        std::path::absolute(core.options.data_dir.join("settings.toml"))
                            .unwrap_or_else(|_| core.options.data_dir.join("settings.toml"))
                            .display()
                            .to_string()
                            .into(),
                    );
                    ui.set_autostart(core.config.autostart);
                    ui.set_show_settings_on_startup(core.config.show_settings_on_startup);
                    ui.set_tray_double_click_action(i32::from(
                        core.config.tray_double_click_settings,
                    ));
                    ui.set_state(display_status(&core.status()).into());
                    ui.set_can_toggle(core.can_toggle());
                    set_saved_config(ui, &core.config);
                }
                ui.set_message(
                    controller
                        .borrow()
                        .as_ref()
                        .and_then(|core| core.shortcut_warning())
                        .unwrap_or("")
                        .into(),
                );
                if let Err(error) = ui.show() {
                    report_error(&options, "設定画面を表示できません", &error.to_string());
                } else {
                    activate_settings(ui);
                    options.marker("open");
                }
            }
        })
    };
    let show_events = show.clone();
    let event_handler = Rc::new(move |event: Event| match event {
        Event::Show => show_events(),
        Event::Hide => {
            if let Some(ui) = settings_events.borrow().as_ref() {
                ui.invoke_cancel_capture();
                let _ = ui.hide();
            }
            options_for_events.marker("closed");
        }
        Event::Toggle => {
            if let Some(core) = controller_events.borrow().as_ref() {
                core.toggle();
            }
        }
        Event::ExerciseSettings => {
            show_events();
            if let Some(ui) = settings_events.borrow().as_ref() {
                ui.set_shortcut("Ctrl+Alt+F10".into());
                ui.invoke_save(
                    ui.get_target(),
                    ui.get_listen(),
                    ui.get_shortcut(),
                    ui.get_autostart(),
                );
                options_for_events.marker("saved");
            }
        }
        Event::Quit => {
            let _ = slint::quit_event_loop();
        }
        Event::ExerciseAutostart => {
            show_events();
            if let Some(ui) = settings_events.borrow().as_ref() {
                let enabled = !ui.get_autostart();
                ui.set_autostart(enabled);
                ui.invoke_save(ui.get_target(), ui.get_listen(), ui.get_shortcut(), enabled);
                options_for_events.marker(if enabled { "startup-on" } else { "startup-off" });
            }
        }
        Event::Status => {
            if let Some(core) = controller_events.borrow().as_ref() {
                let snapshot = core.status();
                let status = snapshot.text();
                let label = display_status(&snapshot);
                let state_known =
                    snapshot.connection == Connection::Connected && snapshot.muted.is_some();
                let can_toggle = core.can_toggle();
                if core.options.benchmark {
                    let _ = std::fs::write(core.options.data_dir.join("ui-status.txt"), &status);
                }
                if let Some(tray) = weak_tray.upgrade() {
                    tray.set_status_text(format!("AlcMute - {label}").into());
                    tray.set_mute_state(if state_known {
                        if snapshot.muted == Some(true) {
                            1
                        } else {
                            2
                        }
                    } else {
                        0
                    });
                    tray.set_can_toggle(can_toggle);
                    if core.options.benchmark {
                        let _ = std::fs::write(
                            core.options.data_dir.join("tray-state.txt"),
                            tray.get_mute_state().to_string(),
                        );
                    }
                }
                if let Some(ui) = settings_events.borrow().as_ref() {
                    ui.set_state(label.into());
                    ui.set_can_toggle(can_toggle);
                }
            }
            // Tray properties use delayed change handlers. Drain them after the
            // latest snapshot is applied, rather than waiting for mouse activity
            // to wake the native notification-area update.
            slint::platform::update_timers_and_animations();
        }
    });
    // The Slint event-loop proxy carries a Send payload. UI handles stay on the UI thread.
    type UiEventHandler = Option<Rc<dyn Fn(Event)>>;
    thread_local! { static HANDLER: RefCell<UiEventHandler> = RefCell::new(None); }
    HANDLER.with(|handler| *handler.borrow_mut() = Some(event_handler));
    let notify = Arc::new(move |event| {
        let _ = slint::invoke_from_event_loop(move || {
            HANDLER.with(|handler| {
                if let Some(callback) = handler.borrow().as_ref() {
                    callback(event);
                }
            })
        });
    });
    *controller.borrow_mut() = Some(Controller::new(options.clone(), notify)?);
    let toggle_controller = controller.clone();
    tray.on_toggle(move || {
        if let Some(core) = toggle_controller.borrow().as_ref() {
            core.toggle();
        }
    });
    let show_double_click = show.clone();
    let click_controller = controller.clone();
    let clicks = RefCell::new(DoubleClick::default());
    tray.on_icon_clicked(move || {
        if !clicks.borrow_mut().click(
            std::time::Instant::now(),
            std::time::Duration::from_millis(alcmute_core::windows::double_click_time() as u64),
        ) {
            return;
        }
        let opens_settings = click_controller
            .borrow()
            .as_ref()
            .is_some_and(|core| core.config.tray_double_click_settings);
        if opens_settings {
            show_double_click();
        } else if let Some(core) = click_controller.borrow().as_ref() {
            core.toggle();
        }
    });
    let startup_show = show.clone();
    let update_show = show.clone();
    let update_check = updates.clone();
    tray.on_updates(move || {
        update_show();
        update_check.check();
    });
    let _activation = alcmute_core::windows::ipc::ActivationServer::start(move |reply| {
        let _ = slint::invoke_from_event_loop(move || {
            HANDLER.with(|handler| {
                if let Some(handler) = handler.borrow().as_ref() {
                    handler(Event::Show);
                }
            });
            reply.acknowledge();
        });
    })?;
    tray.on_settings(move || show());
    tray.on_about(move || show_about());
    tray.on_exit_requested(|| {
        let _ = slint::quit_event_loop();
    });
    tray.show().map_err(|e| e.to_string())?;
    options.marker("ready");
    if !options.benchmark
        && !std::env::args().any(|arg| arg == "--background")
        && controller.borrow().as_ref().is_some_and(|core| {
            core.config.show_settings_on_startup || core.shortcut_warning().is_some()
        })
    {
        startup_show();
    }
    updates.start();
    let result = slint::run_event_loop_until_quit().map_err(|e| e.to_string());
    controller.borrow_mut().take();
    HANDLER.with(|handler| handler.borrow_mut().take());
    result
}

fn display_status(status: &alcmute_core::Status) -> String {
    let label = match status.connection {
        Connection::Disconnected => "VRChatは起動していません",
        Connection::Unavailable => "状態不明：OSCQueryに接続できません",
        Connection::Checking => "VRChatの状態を確認中…",
        Connection::Connected => match status.muted {
            Some(true) => "ミュート中",
            Some(false) => "マイク有効",
            None => "状態を受信中…",
        },
    };
    if status.detail.is_empty() {
        label.into()
    } else {
        format!("{label} ({})", status.detail)
    }
}

fn activate_settings(ui: &Settings) {
    if ui.window().is_minimized() {
        ui.window().set_minimized(false);
    }
    ui.window().with_winit_window(|window| {
        window.focus_window();
    });
}

#[derive(Default)]
struct DoubleClick {
    last: Option<std::time::Instant>,
}
impl DoubleClick {
    fn click(&mut self, now: std::time::Instant, interval: std::time::Duration) -> bool {
        if self
            .last
            .is_some_and(|last| now.duration_since(last) <= interval)
        {
            self.last = None;
            true
        } else {
            self.last = Some(now);
            false
        }
    }
}

#[derive(Default)]
struct CaptureState {
    active: bool,
    armed: bool,
    finishing: bool,
    original: String,
    pending_shortcut: Option<String>,
}
impl CaptureState {
    fn stage_shortcut(&mut self, value: String) -> bool {
        let count = value.split('+').count();
        if self
            .pending_shortcut
            .as_ref()
            .is_none_or(|old| old.split('+').count() <= count)
        {
            self.pending_shortcut = Some(value);
            true
        } else {
            false
        }
    }
}
fn fill_config(ui: &Settings, config: &Config) {
    ui.set_target(config.target.clone().into());
    ui.set_listen(config.listen.clone().into());
    ui.set_shortcut(config.shortcut.clone().into());
    ui.set_shortcut_label(shortcut::label(&config.shortcut).into());
    ui.set_autostart(config.autostart);
    ui.set_show_settings_on_startup(config.show_settings_on_startup);
    ui.set_tray_double_click_action(i32::from(config.tray_double_click_settings));
    set_saved_config(ui, config);
}

fn set_saved_config(ui: &Settings, config: &Config) {
    ui.set_saved_target(config.target.clone().into());
    ui.set_saved_listen(config.listen.clone().into());
    ui.set_saved_shortcut(config.shortcut.clone().into());
    ui.set_saved_autostart(config.autostart);
    ui.set_saved_show_settings_on_startup(config.show_settings_on_startup);
    ui.set_saved_tray_double_click_action(i32::from(config.tray_double_click_settings));
}
fn restore_capture_hotkey(ui: &Settings, controller: &Rc<RefCell<Option<Controller>>>) {
    if let Some(core) = controller.borrow_mut().as_mut() {
        if let Err(error) = core.end_shortcut_capture() {
            ui.set_save_ok(false);
            ui.set_message(error.into());
        }
    }
}
fn install_editing(ui: &Settings, controller: Rc<RefCell<Option<Controller>>>, options: Options) {
    ui.window().on_winit_window_event(|_, event| {
        use slint::winit_030::winit::{
            event::{ElementState, WindowEvent},
            keyboard::PhysicalKey,
        };
        match event {
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    alcmute_core::windows::foreground_key_event(
                        &format!("{code:?}"),
                        event.state == ElementState::Pressed,
                    );
                }
            }
            WindowEvent::Focused(true) => alcmute_core::windows::foreground_focus_gained(),
            _ => {}
        }
        slint::winit_030::EventResult::Propagate
    });
    let capture = Rc::new(RefCell::new(CaptureState::default()));
    let polling = Arc::new(std::sync::atomic::AtomicU64::new(0));

    let weak = ui.as_weak();
    let state = capture.clone();
    let core = controller.clone();
    let polling_start = polling.clone();
    ui.on_begin_capture(move || {
        if state.borrow().active {
            return;
        }
        let result = core
            .borrow_mut()
            .as_mut()
            .ok_or("アプリが終了しています".to_string())
            .and_then(Controller::begin_shortcut_capture);
        if let Some(ui) = weak.upgrade() {
            match result {
                Ok(()) => {
                    *state.borrow_mut() = CaptureState {
                        active: true,
                        armed: true,
                        original: ui.get_shortcut().to_string(),
                        ..Default::default()
                    };
                    ui.set_capture_hint(
                        "登録するキーの組み合わせを押してください（Escでキャンセル）".into(),
                    );
                    ui.set_capturing(true);
                    ui.set_message("".into());
                    // Slint's secondary-window timer context does not reliably tick
                    // in this tray-only event loop. Wake the main loop explicitly.
                    let generation =
                        polling_start.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    let polling = polling_start.clone();
                    let weak = ui.as_weak();
                    std::thread::spawn(move || {
                        while polling.load(std::sync::atomic::Ordering::Relaxed) == generation {
                            let weak = weak.clone();
                            if slint::invoke_from_event_loop(move || {
                                if let Some(ui) = weak.upgrade() {
                                    ui.invoke_capture_key();
                                }
                            })
                            .is_err()
                            {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(20));
                        }
                    });
                }
                Err(error) => {
                    ui.set_save_ok(false);
                    ui.set_message(error.into());
                }
            }
        }
    });
    let weak = ui.as_weak();
    let state = capture.clone();
    let core = controller.clone();
    let polling_finish = polling.clone();
    ui.on_capture_key(move || {
        let Some(ui) = weak.upgrade() else {
            return;
        };
        let physical = alcmute_core::windows::pressed_keyboard_keys();
        let keys =
            alcmute_core::windows::recorded_keyboard_keys().unwrap_or_else(|| physical.clone());
        let mut state = state.borrow_mut();
        if !state.active {
            return;
        }
        if !alcmute_core::windows::foreground_is_ours() || keys.contains(&0x1b) {
            drop(state);
            ui.invoke_cancel_capture();
            return;
        }
        if !keys.is_empty() && state.armed && !state.finishing {
            match shortcut::from_keys(&keys) {
                Ok(Some(value)) => {
                    if state.stage_shortcut(value.clone()) {
                        ui.set_capture_hint(
                            format!(
                                "{}：すべてのキーを離すと確定します",
                                shortcut::label(&value)
                            )
                            .into(),
                        );
                    }
                }
                Ok(None) => {}
                Err(error) => {
                    state.pending_shortcut = None;
                    ui.set_capture_hint(error.into());
                }
            }
        }
        if physical.is_empty() {
            state.armed = true;
            if !state.finishing {
                if let Some(value) = state.pending_shortcut.take() {
                    ui.set_shortcut_label(shortcut::label(&value).into());
                    ui.set_shortcut(value.into());
                    state.finishing = true;
                }
            }
            if state.finishing {
                state.active = false;
                polling_finish.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                ui.set_capturing(false);
                drop(state);
                restore_capture_hotkey(&ui, &core);
            }
        }
    });
    let weak = ui.as_weak();
    let state = capture.clone();
    let core = controller.clone();
    ui.on_cancel_capture(move || {
        let mut state = state.borrow_mut();
        if !state.active {
            drop(state);
            if let Some(ui) = weak.upgrade() {
                restore_capture_hotkey(&ui, &core);
            }
            return;
        }
        state.active = false;
        polling.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        state.finishing = false;
        let original = state.original.clone();
        drop(state);
        if let Some(ui) = weak.upgrade() {
            ui.set_shortcut(original.clone().into());
            ui.set_shortcut_label(shortcut::label(&original).into());
            ui.set_capturing(false);
            ui.set_capture_hint("キャンセルしました".into());
            restore_capture_hotkey(&ui, &core);
        }
    });
    let weak = ui.as_weak();
    ui.on_reset_field(move |field| {
        if let Some(ui) = weak.upgrade() {
            let defaults = Config::default();
            match field {
                0 => ui.set_target(defaults.target.into()),
                1 => ui.set_listen(defaults.listen.into()),
                2 => {
                    ui.set_shortcut_label(shortcut::label(&defaults.shortcut).into());
                    ui.set_shortcut(defaults.shortcut.into());
                }
                3 => ui.set_autostart(false),
                4 => ui.set_tray_double_click_action(0),
                5 => ui.set_show_settings_on_startup(true),
                _ => return,
            }
            ui.set_save_ok(true);
            ui.set_message("デフォルト値に戻しました。「保存」で反映されます".into());
        }
    });
    let weak = ui.as_weak();
    ui.on_reset_all(move || {
        if !alcmute_core::windows::confirm_settings_reset() {
            return;
        }
        let result = controller
            .borrow_mut()
            .as_mut()
            .ok_or("アプリが終了しています".to_string())
            .and_then(Controller::reset_to_defaults);
        if let Some(ui) = weak.upgrade() {
            ui.set_save_ok(result.is_ok());
            match result {
                Ok(()) => {
                    fill_config(&ui, &Config::default());
                    ui.invoke_update_preferences(true);
                    ui.set_message("保存済み設定を削除し、自動起動を解除しました".into());
                    options.marker("reset");
                }
                Err(error) => ui.set_message(error.into()),
            }
        }
    });
}

#[cfg(test)]
mod double_click_tests {
    use super::*;
    #[test]
    fn recording_keeps_the_full_chord_when_keys_are_released_separately() {
        let mut capture = CaptureState::default();
        assert!(capture.stage_shortcut("KeyK".into()));
        assert!(capture.stage_shortcut("KeyK+KeyL".into()));
        assert!(!capture.stage_shortcut("KeyL".into()));
        assert_eq!(capture.pending_shortcut.as_deref(), Some("KeyK+KeyL"));
    }
    #[test]
    fn dirty_tracks_edit_revert_save_and_reset_for_every_setting() {
        let ui = Settings::new().unwrap();
        let original = Config::default();
        fill_config(&ui, &original);
        assert!(!ui.get_dirty());
        assert!(ui.get_double_click_toggle_selected());
        assert!(!ui.get_double_click_settings_selected());
        ui.set_target("127.0.0.1:9010".into());
        assert!(ui.get_dirty());
        ui.set_target(original.target.clone().into());
        assert!(!ui.get_dirty());
        ui.set_listen("127.0.0.1:9011".into());
        assert!(ui.get_dirty());
        fill_config(&ui, &original);
        ui.set_shortcut("RightCtrl+RightAlt".into());
        assert!(ui.get_dirty());
        fill_config(&ui, &original);
        ui.set_autostart(true);
        assert!(ui.get_dirty());
        fill_config(&ui, &original);
        ui.set_show_settings_on_startup(false);
        assert!(ui.get_dirty());
        fill_config(&ui, &original);
        ui.set_tray_double_click_action(1);
        assert!(ui.get_dirty());
        assert!(!ui.get_double_click_toggle_selected());
        assert!(ui.get_double_click_settings_selected());
        let saved = Config {
            tray_double_click_settings: true,
            ..original.clone()
        };
        set_saved_config(&ui, &saved);
        assert!(!ui.get_dirty());
        ui.set_tray_double_click_action(0);
        assert!(ui.get_dirty());
        fill_config(&ui, &original);
        assert!(!ui.get_dirty());
        assert!(ui.get_double_click_toggle_selected());
        assert!(!ui.get_double_click_settings_selected());
        // A full reset must also reset the actual radio presentation after a
        // saved 'open settings' selection, not merely the persisted flag.
        fill_config(&ui, &saved);
        assert!(ui.get_double_click_settings_selected());
        fill_config(&ui, &Config::default());
        slint::platform::update_timers_and_animations();
        assert_eq!(ui.get_tray_double_click_action(), 0);
        assert!(ui.get_double_click_toggle_selected());
        assert!(!ui.get_double_click_settings_selected());
        assert!(!ui.get_dirty());
    }
    #[test]
    fn pairs_clicks_without_turning_a_triple_click_into_two_actions() {
        let mut clicks = DoubleClick::default();
        let now = std::time::Instant::now();
        let interval = std::time::Duration::from_millis(400);
        assert!(!clicks.click(now, interval));
        assert!(clicks.click(now + interval, interval));
        assert!(!clicks.click(now + interval + interval, interval));
        assert!(!clicks.click(now + std::time::Duration::from_secs(2), interval));
    }
}
