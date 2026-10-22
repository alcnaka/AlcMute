use slint::{winit_030::WinitWindowAccessor, ComponentHandle};
use std::{cell::RefCell, rc::Rc};

slint::slint! {
    import { VerticalBox, HorizontalBox, Button, TextEdit, ComboBox } from "std-widgets.slint";
    export component About inherits Window {
        title: "AlcMuteについて";
        icon: @image-url("../assets/app.svg");
        preferred-width: 640px; preferred-height: 640px;
        min-width: 420px; min-height: 400px;
        default-font-size: 15px;
        in property <string> version;
        in property <string> mit-license;
        in property <string> dependency-licenses;
        in property <[string]> libraries;
        in-out property <int> library-index: 0;
        in property <int> page-index;
        in property <int> page-count;
        in-out property <bool> show-dependencies: false;
        in property <string> error-message;
        callback repository();
        callback select-library(int);
        callback turn-page(int);
        callback dismiss();
        VerticalBox {
            padding: 20px; spacing: 12px;
            Text { text: "AlcMute " + root.version; font-size: 26px; font-weight: 700; }
            Text { text: "あるみゅーと / VRC Background Mute"; }
            Text { text: "制作者：Alcnaka"; }
            Button { text: "github.com/alcnaka/AlcMute を開く"; clicked => { root.repository(); } }
            Text { text: "ライセンス：MIT（AlcMuteの独自コード）"; wrap: word-wrap; }
            HorizontalBox {
                Button { text: "MITライセンス本文"; primary: !root.show-dependencies; clicked => { root.show-dependencies = false; } }
                Button { text: "依存ライブラリのライセンス"; primary: root.show-dependencies; clicked => { root.show-dependencies = true; } }
            }
            if root.show-dependencies: ComboBox {
                model: root.libraries;
                current-index <=> root.library-index;
                selected => { root.select-library(self.current-index); }
            }
            TextEdit {
                text: root.show-dependencies ? root.dependency-licenses : root.mit-license;
                read-only: true;
                wrap: word-wrap;
                vertical-stretch: 1;
            }
            if root.show-dependencies: HorizontalBox {
                Button { text: "前のページ"; enabled: root.page-index > 0; clicked => { root.turn-page(-1); } }
                Text { text: (root.page-index + 1) + " / " + root.page-count; vertical-alignment: center; horizontal-alignment: center; }
                Button { text: "次のページ"; enabled: root.page-index + 1 < root.page-count; clicked => { root.turn-page(1); } }
            }
            if root.error-message != "": Text { text: root.error-message; wrap: word-wrap; }
            HorizontalBox {
                alignment: end;
                Button { text: "閉じる"; clicked => { root.dismiss(); } }
            }
        }
    }
}

pub fn show(slot: &Rc<RefCell<Option<About>>>) -> Result<(), String> {
    if slot.borrow().is_none() {
        let ui = About::new().map_err(|e| e.to_string())?;
        ui.set_version(env!("CARGO_PKG_VERSION").into());
        ui.set_mit_license(include_str!("../LICENSE").into());
        // Keep the full document in static storage, never in a text widget.
        let browser = Rc::new(RefCell::new(LicenseBrowser::new()));
        ui.set_libraries(slint::ModelRc::new(slint::VecModel::from(
            browser
                .borrow()
                .libraries
                .iter()
                .map(|text| slint::SharedString::from(text.lines().next().unwrap_or("License")))
                .collect::<Vec<_>>(),
        )));
        browser.borrow().update(&ui);
        let weak = ui.as_weak();
        let select_browser = browser.clone();
        ui.on_select_library(move |index| {
            if let Some(ui) = weak.upgrade() {
                let mut browser = select_browser.borrow_mut();
                if let Some(text) = browser.libraries.get(index as usize) {
                    browser.pages = pages(text);
                    browser.page = 0;
                    browser.update(&ui);
                }
            }
        });
        let weak = ui.as_weak();
        ui.on_turn_page(move |direction| {
            if let Some(ui) = weak.upgrade() {
                let mut browser = browser.borrow_mut();
                browser.page = browser
                    .page
                    .saturating_add_signed(direction as isize)
                    .min(browser.pages.len() - 1);
                browser.update(&ui);
            }
        });
        let weak = ui.as_weak();
        ui.on_repository(move || {
            if let Err(error) = std::process::Command::new("explorer.exe")
                .arg("https://github.com/alcnaka/AlcMute")
                .spawn()
            {
                if let Some(ui) = weak.upgrade() {
                    ui.set_error_message(format!("リンクを開けませんでした：{error}").into());
                }
            }
        });
        let weak = ui.as_weak();
        ui.on_dismiss(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.hide();
            }
        });
        let weak = ui.as_weak();
        ui.window().on_close_requested(move || {
            if let Some(ui) = weak.upgrade() {
                let _ = ui.hide();
            }
            slint::CloseRequestResponse::KeepWindowShown
        });
        *slot.borrow_mut() = Some(ui);
    }
    if let Some(ui) = slot.borrow().as_ref() {
        ui.show().map_err(|e| e.to_string())?;
        if ui.window().is_minimized() {
            ui.window().set_minimized(false);
        }
        ui.window()
            .with_winit_window(|window| window.focus_window());
    }
    Ok(())
}

const NOTICES: &str = include_str!("../assets/dependency-licenses.txt");
const SEPARATOR: &str = "\n========================================\n";

struct LicenseBrowser {
    libraries: Vec<&'static str>,
    pages: Vec<&'static str>,
    page: usize,
}

impl LicenseBrowser {
    fn new() -> Self {
        let libraries: Vec<_> = NOTICES.split(SEPARATOR).skip(1).collect();
        let first = libraries.first().copied().unwrap_or(NOTICES);
        Self {
            libraries,
            pages: pages(first),
            page: 0,
        }
    }

    fn update(&self, ui: &About) {
        ui.set_dependency_licenses(self.pages[self.page].into());
        ui.set_page_index(self.page as i32);
        ui.set_page_count(self.pages.len() as i32);
    }
}

// Limit both characters and lines, even for unbroken lines or tiny windows.
// Splitting on UTF-8 character boundaries preserves the complete original text.
fn pages(text: &str) -> Vec<&str> {
    let mut result = Vec::new();
    let (mut start, mut characters, mut lines) = (0, 0, 0);
    for (index, character) in text.char_indices() {
        if characters == 2048 || lines == 48 {
            result.push(&text[start..index]);
            start = index;
            characters = 0;
            lines = 0;
        }
        characters += 1;
        lines += usize::from(character == '\n');
    }
    result.push(&text[start..]);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_license_is_lossless_and_bounded() {
        let browser = LicenseBrowser::new();
        assert!(browser.libraries.len() > 100);
        for library in browser.libraries {
            let chunks = pages(library);
            assert_eq!(chunks.concat(), library);
            for chunk in chunks {
                assert!(chunk.chars().count() <= 2048);
                assert!(chunk.chars().filter(|c| *c == '\n').count() <= 48);
            }
        }
    }

    #[test]
    fn pathological_text_and_utf8_remain_readable() {
        for text in [String::new(), "あ😀".repeat(10000), "\n".repeat(10000)] {
            let chunks = pages(&text);
            assert!(!chunks.is_empty());
            assert_eq!(chunks.concat(), text);
            assert!(chunks.iter().all(|p| p.chars().count() <= 2048));
        }
    }
}
