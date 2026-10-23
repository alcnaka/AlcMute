use crate::{about, AppTheme, Settings};
use alcmute_core::{appearance::Theme, Config, Controller, Options};
use slint::{
    winit_030::{winit::window::Theme as WindowTheme, WinitWindowAccessor},
    ComponentHandle,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

#[derive(Clone)]
pub struct Appearance(Rc<State>);
struct State {
    saved: Cell<Theme>,
    preview: Cell<Theme>,
    settings: RefCell<Option<slint::Weak<Settings>>>,
    about: RefCell<Option<slint::Weak<about::About>>>,
}
fn native_theme(theme: Theme) -> Option<WindowTheme> {
    match theme {
        Theme::Auto => None,
        Theme::Light => Some(WindowTheme::Light),
        Theme::Dark => Some(WindowTheme::Dark),
    }
}
impl Appearance {
    pub fn new(options: Options) -> Result<Self, String> {
        let theme = Config::load(&options)?.theme;
        Ok(Self(Rc::new(State {
            saved: Cell::new(theme),
            preview: Cell::new(theme),
            settings: RefCell::new(None),
            about: RefCell::new(None),
        })))
    }
    pub fn attach(&self, ui: &Settings) {
        *self.0.settings.borrow_mut() = Some(ui.as_weak());
        self.restore();
        let service = self.clone();
        ui.on_preview_theme(move |index| {
            if let Ok(theme) = Theme::from_index(index) {
                service.preview(theme);
            }
        });
        let service = self.clone();
        ui.on_reset_theme(move || {
            service.accept_saved(Theme::Auto);
            service.restore();
            true
        });
    }
    pub fn attach_about(&self, ui: &about::About) {
        *self.0.about.borrow_mut() = Some(ui.as_weak());
        self.apply_current();
    }
    pub fn preview(&self, theme: Theme) {
        self.0.preview.set(theme);
        self.apply_current();
    }
    pub fn apply_current(&self) {
        let theme = self.0.preview.get();
        if let Some(ui) = self
            .0
            .settings
            .borrow()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            ui.global::<AppTheme>().invoke_apply(theme.index());
            ui.window()
                .with_winit_window(|window| window.set_theme(native_theme(theme)));
        }
        if let Some(ui) = self
            .0
            .about
            .borrow()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            ui.global::<about::AppTheme>().invoke_apply(theme.index());
            ui.window()
                .with_winit_window(|window| window.set_theme(native_theme(theme)));
        }
    }
    pub fn accept_saved(&self, theme: Theme) {
        self.0.saved.set(theme);
        if let Some(ui) = self
            .0
            .settings
            .borrow()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            ui.set_saved_theme_index(theme.index());
        }
    }
    pub fn restore(&self) {
        let theme = self.0.saved.get();
        if let Some(ui) = self
            .0
            .settings
            .borrow()
            .as_ref()
            .and_then(slint::Weak::upgrade)
        {
            ui.set_theme_index(theme.index());
            ui.set_saved_theme_index(theme.index());
        }
        self.preview(theme);
    }
}

/// All durable settings are committed by the Controller's existing transaction.
pub fn save_settings(
    core: &mut Controller,
    updates: &crate::updates::Updates,
    appearance: &Appearance,
    mut config: Config,
    auto_check: bool,
    theme_index: i32,
) -> Result<(), String> {
    let theme = Theme::from_index(theme_index)?;
    config.auto_check_updates = auto_check;
    config.theme = theme;
    core.apply(config).map_err(|error| error.to_string())?;
    updates.accept_saved(auto_check);
    appearance.accept_saved(theme);
    Ok(())
}
