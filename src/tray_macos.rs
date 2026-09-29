// macOS Menu Bar (NSStatusItem) Service for Minkey
// Matches OpenKey AppDelegate.m status item behavior: "V" / "E" title + the same menu as the Windows tray.
// NSStatusItem must live on the main thread, so it is created inside the Slint event loop.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::hook::HookConfig;
use crate::tray::TrayCallbacks;
use crate::types::{CodeTable, InputType};
use crate::VietnameseEngine;

const ID_VIET_ON_OFF: &str = "viet_on_off";
const ID_SPELLING: &str = "spelling";
const ID_SMART_SWITCH: &str = "smart_switch";
const ID_USE_MACRO: &str = "use_macro";
const ID_TELEX: &str = "telex";
const ID_VNI: &str = "vni";
const ID_SIMPLE_TELEX: &str = "simple_telex";
const ID_UNICODE: &str = "unicode";
const ID_TCVN3: &str = "tcvn3";
const ID_VNI_WINDOWS: &str = "vni_windows";
const ID_UNICODE_COMPOUND: &str = "unicode_compound";
const ID_CP1258: &str = "cp1258";
const ID_MACRO_TABLE: &str = "macro_table";
const ID_CONVERT_TOOL: &str = "convert_tool";
const ID_CONTROL_PANEL: &str = "control_panel";
const ID_ABOUT: &str = "about";
const ID_EXIT: &str = "exit";

/// Thread-safe state shared between the menu event handler and the service
struct Shared {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    callbacks: Arc<TrayCallbacks>,
}

static SHARED: OnceLock<Shared> = OnceLock::new();

/// Main-thread-only objects
struct StatusItem {
    tray: TrayIcon,
    viet: CheckMenuItem,
    spelling: CheckMenuItem,
    smart_switch: CheckMenuItem,
    use_macro: CheckMenuItem,
    telex: CheckMenuItem,
    vni: CheckMenuItem,
    simple_telex: CheckMenuItem,
    unicode: CheckMenuItem,
    tcvn3: CheckMenuItem,
    vni_windows: CheckMenuItem,
    unicode_compound: CheckMenuItem,
    cp1258: CheckMenuItem,
}

thread_local! {
    static STATUS_ITEM: RefCell<Option<StatusItem>> = const { RefCell::new(None) };
}

fn check(id: &str, text: &str) -> CheckMenuItem {
    CheckMenuItem::with_id(id, text, true, false, None)
}

fn build_status_item() -> Option<StatusItem> {
    let item = StatusItem {
        tray: TrayIconBuilder::new().with_title("V").with_tooltip("Minkey").build().ok()?,
        viet: check(ID_VIET_ON_OFF, "Bật Tiếng Việt"),
        spelling: check(ID_SPELLING, "Bật kiểm tra chính tả"),
        smart_switch: check(ID_SMART_SWITCH, "Bật loại trừ ứng dụng thông minh"),
        use_macro: check(ID_USE_MACRO, "Bật gõ tắt"),
        telex: check(ID_TELEX, "Kiểu gõ Telex"),
        vni: check(ID_VNI, "Kiểu gõ VNI"),
        simple_telex: check(ID_SIMPLE_TELEX, "Kiểu gõ Simple Telex"),
        unicode: check(ID_UNICODE, "Unicode dựng sẵn"),
        tcvn3: check(ID_TCVN3, "TCVN3 (ABC)"),
        vni_windows: check(ID_VNI_WINDOWS, "VNI Windows"),
        unicode_compound: check(ID_UNICODE_COMPOUND, "Unicode tổ hợp"),
        cp1258: check(ID_CP1258, "Vietnamese locale CP 1258"),
    };

    let other_code = Submenu::new("Bảng mã khác", true);
    let _ = other_code.append_items(&[&item.unicode_compound, &item.cp1258]);

    let sep = PredefinedMenuItem::separator;
    let menu = Menu::new();
    let _ = menu.append_items(&[
        &item.viet,
        &sep(),
        &item.spelling,
        &item.smart_switch,
        &item.use_macro,
        &sep(),
        &MenuItem::with_id(ID_MACRO_TABLE, "Cấu hình gõ tắt...", true, None),
        &MenuItem::with_id(ID_CONVERT_TOOL, "Công cụ chuyển mã...", true, None),
        &sep(),
        &item.telex,
        &item.vni,
        &item.simple_telex,
        &sep(),
        &item.unicode,
        &item.tcvn3,
        &item.vni_windows,
        &other_code,
        &sep(),
        &MenuItem::with_id(ID_CONTROL_PANEL, "Bảng điều khiển...", true, None),
        &MenuItem::with_id(ID_ABOUT, "Giới thiệu Minkey", true, None),
        &sep(),
        &MenuItem::with_id(ID_EXIT, "Thoát Minkey", true, None),
    ]);
    item.tray.set_menu(Some(Box::new(menu)));
    Some(item)
}

/// Syncs the "V"/"E" title and all check marks with the current settings (main thread only)
fn refresh_now() {
    let Some(shared) = SHARED.get() else { return };
    let is_viet = shared.config.language.load(Ordering::Relaxed) != 0;
    let smart_switch = shared.config.use_smart_switch_key.load(Ordering::Relaxed);
    let (input_type, code_table, check_spelling, use_macro) = match shared.engine.lock() {
        Ok(e) => (e.input_type, e.code_table, e.check_spelling, e.use_macro),
        Err(_) => return,
    };

    STATUS_ITEM.with(|slot| {
        let slot = slot.borrow();
        let Some(item) = slot.as_ref() else { return };
        item.tray.set_title(Some(if is_viet { "V" } else { "E" }));
        let _ = item.tray.set_tooltip(Some(if is_viet { "Minkey - Tiếng Việt" } else { "Minkey - English" }));
        item.viet.set_checked(is_viet);
        item.spelling.set_checked(check_spelling);
        item.smart_switch.set_checked(smart_switch);
        item.use_macro.set_checked(use_macro);
        item.telex.set_checked(input_type == InputType::Telex);
        item.vni.set_checked(input_type == InputType::Vni);
        item.simple_telex
            .set_checked(matches!(input_type, InputType::SimpleTelex1 | InputType::SimpleTelex2));
        item.unicode.set_checked(code_table == 0);
        item.tcvn3.set_checked(code_table == 1);
        item.vni_windows.set_checked(code_table == 2);
        item.unicode_compound.set_checked(code_table == 3);
        item.cp1258.set_checked(code_table == 4);
    });
}

/// Schedules a menu bar refresh on the main thread. Safe to call from any thread.
pub fn request_refresh() {
    let _ = slint::invoke_from_event_loop(refresh_now);
}

fn handle_menu_event(event: MenuEvent) {
    let Some(shared) = SHARED.get() else { return };
    let id = event.id().as_ref();

    let set_input_type = |t: InputType| {
        if let Ok(mut engine) = shared.engine.lock() {
            engine.input_type = t;
            engine.start_new_session();
        }
    };
    let set_code_table = |t: CodeTable| {
        if let Ok(mut engine) = shared.engine.lock() {
            engine.code_table = t.to_u32() as usize;
        }
    };

    match id {
        ID_VIET_ON_OFF => {
            let next = if shared.config.language.load(Ordering::SeqCst) == 0 { 1 } else { 0 };
            shared.config.language.store(next, Ordering::SeqCst);
            if let Ok(mut engine) = shared.engine.lock() {
                engine.language = next;
                engine.start_new_session();
            }
        }
        ID_SPELLING => {
            if let Ok(mut engine) = shared.engine.lock() {
                let cur = engine.check_spelling;
                engine.set_check_spelling(!cur);
            }
        }
        ID_SMART_SWITCH => {
            let cur = shared.config.use_smart_switch_key.load(Ordering::Relaxed);
            shared.config.use_smart_switch_key.store(!cur, Ordering::Relaxed);
        }
        ID_USE_MACRO => {
            if let Ok(mut engine) = shared.engine.lock() {
                engine.use_macro = !engine.use_macro;
            }
        }
        ID_TELEX => set_input_type(InputType::Telex),
        ID_VNI => set_input_type(InputType::Vni),
        ID_SIMPLE_TELEX => set_input_type(InputType::SimpleTelex1),
        ID_UNICODE => set_code_table(CodeTable::Unicode),
        ID_TCVN3 => set_code_table(CodeTable::Tcvn3),
        ID_VNI_WINDOWS => set_code_table(CodeTable::VniWindows),
        ID_UNICODE_COMPOUND => set_code_table(CodeTable::UnicodeCompound),
        ID_CP1258 => set_code_table(CodeTable::Cp1258),
        ID_MACRO_TABLE => (shared.callbacks.on_open_macro_table)(),
        ID_CONVERT_TOOL => (shared.callbacks.on_open_convert_tool)(),
        ID_CONTROL_PANEL => (shared.callbacks.on_open_control_panel)(),
        ID_ABOUT => (shared.callbacks.on_open_about)(),
        ID_EXIT => (shared.callbacks.on_exit)(),
        _ => {}
    }
    // CheckMenuItem toggles itself on click; re-sync with the real state
    request_refresh();
}

pub struct MacTrayService {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    callbacks: Arc<TrayCallbacks>,
}

impl MacTrayService {
    pub fn new(config: HookConfig, engine: Arc<Mutex<VietnameseEngine>>, callbacks: TrayCallbacks) -> Self {
        Self { config, engine, callbacks: Arc::new(callbacks) }
    }

    pub fn start(&mut self, _use_gray_icon: Arc<AtomicBool>) {
        let _ = SHARED.set(Shared {
            config: self.config.clone(),
            engine: self.engine.clone(),
            callbacks: self.callbacks.clone(),
        });

        MenuEvent::set_event_handler(Some(handle_menu_event));
        // Clicking the status item: make sure check marks are current before the menu opens
        TrayIconEvent::set_event_handler(Some(|_event: TrayIconEvent| refresh_now()));

        // Create the status item once the Slint (NSApplication) event loop is running
        slint::Timer::single_shot(std::time::Duration::ZERO, || {
            crate::macos_app::set_accessory_app();
            STATUS_ITEM.with(|slot| *slot.borrow_mut() = build_status_item());
            refresh_now();
        });
    }

    pub fn stop(&mut self) {
        MenuEvent::set_event_handler(None::<fn(MenuEvent)>);
        TrayIconEvent::set_event_handler(None::<fn(TrayIconEvent)>);
        STATUS_ITEM.with(|slot| slot.borrow_mut().take());
    }

    pub fn get_hwnd(&self) -> usize {
        0 // No Win32 HWND on macOS
    }
}
