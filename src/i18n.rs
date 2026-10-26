use alcmute_core::language::{self, Language};
use slint::ComponentHandle;
use std::cell::RefCell;
type Observer = Box<dyn Fn(bool)>;
thread_local! { static OBSERVERS: RefCell<Vec<Observer>> = RefCell::new(Vec::new()); }
macro_rules! attach {
    ($name:ident, $ui:ty, $global:ty) => {
        pub fn $name(ui: &$ui) {
            ui.global::<$global>()
                .on_translate(|value| language::translate_en(&value).into());
            ui.global::<$global>().set_english(language::english());
            let weak = ui.as_weak();
            OBSERVERS.with(|observers| {
                observers.borrow_mut().push(Box::new(move |english| {
                    if let Some(ui) = weak.upgrade() {
                        ui.global::<$global>().set_english(english);
                    }
                }))
            });
        }
    };
}
attach!(attach_settings, crate::Settings, crate::I18n);
attach!(attach_tray, crate::Tray, crate::I18n);
attach!(attach_about, crate::about::About, crate::about::I18n);
pub fn apply(language: Language) {
    language::set(language);
    OBSERVERS.with(|observers| {
        for observer in observers.borrow().iter() {
            observer(language::english());
        }
    });
}
