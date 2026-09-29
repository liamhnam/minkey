// Minkey - Modern Open-Source Vietnamese Input Method (Cross-Platform Rust Port of OpenKey)
// Entry point, Lazy UI manager, and service coordinator

#![cfg_attr(windows, windows_subsystem = "windows")]

use std::cell::{OnceCell, RefCell};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use slint::ComponentHandle;

#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use minkey::config::AppConfig;
use minkey::engine::VietnameseEngine;
use minkey::hook::PlatformHookService;
use minkey::tray::{PlatformTrayService, TrayCallbacks};
use minkey::types::InputType;
use minkey::ui::{
    populate_convert_ui_from_config, populate_ui_from_config, setup_convert_window_callbacks,
    setup_macro_window_callbacks, setup_ui_callbacks, update_macro_window_items,
    ConvertWindow, MainWindow, MacroWindow,
};

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn CreateMutexW(
        lp_mutex_attributes: *const std::ffi::c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> HANDLE;
}

#[cfg(windows)]
fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn sync_engine_from_config(engine: &mut VietnameseEngine, config: &AppConfig) {
    engine.language = config.language.load(Ordering::Relaxed);
    engine.input_type = match config.input_type.load(Ordering::Relaxed) {
        0 => InputType::Telex,
        1 => InputType::Vni,
        2 => InputType::SimpleTelex1,
        3 => InputType::SimpleTelex2,
        _ => InputType::Telex,
    };
    engine.code_table = config.code_table.load(Ordering::Relaxed) as usize;
    engine.check_spelling = config.check_spelling.load(Ordering::Relaxed);
    engine.use_modern_orthography = config.use_modern_orthography.load(Ordering::Relaxed);
    engine.quick_telex = config.quick_telex.load(Ordering::Relaxed);
    engine.restore_if_wrong_spelling = config.restore_if_wrong_spelling.load(Ordering::Relaxed);
    engine.use_macro = config.use_macro.load(Ordering::Relaxed);
    engine.use_macro_in_english = config.use_macro_in_english.load(Ordering::Relaxed);
    engine.auto_caps_macro = config.auto_caps_macro.load(Ordering::Relaxed);
    engine.upper_case_first_char = config.upper_case_first_char.load(Ordering::Relaxed);
    engine.allow_consonant_zfwj = config.allow_consonant_zfwj.load(Ordering::Relaxed);
    engine.quick_start_consonant = config.quick_start_consonant.load(Ordering::Relaxed);
    engine.quick_end_consonant = config.quick_end_consonant.load(Ordering::Relaxed);
}

/// Creates UI windows on first use. Slint components are not `Send`, so the manager lives in a
/// main-thread `thread_local` and other threads reach it through `slint::invoke_from_event_loop`.
struct LazyWindowManager {
    main_window: RefCell<Option<MainWindow>>,
    macro_window: RefCell<Option<MacroWindow>>,
    convert_window: RefCell<Option<ConvertWindow>>,
    app_config: Arc<AppConfig>,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<minkey::smart_switch::SmartSwitchTable>>,
    tray_hwnd: usize,
}

thread_local! {
    static WINDOW_MANAGER: OnceCell<LazyWindowManager> = const { OnceCell::new() };
}

/// Runs `f` with the window manager (main thread only; no-op before it is installed)
fn with_wm(f: impl FnOnce(&LazyWindowManager)) {
    WINDOW_MANAGER.with(|cell| {
        if let Some(wm) = cell.get() {
            f(wm);
        }
    });
}

/// Schedules `f` on the UI thread with the window manager. Callable from any thread.
fn on_ui_thread(f: impl FnOnce(&LazyWindowManager) + Send + 'static) {
    let _ = slint::invoke_from_event_loop(move || with_wm(f));
}

impl LazyWindowManager {
    fn new(
        app_config: Arc<AppConfig>,
        engine: Arc<Mutex<VietnameseEngine>>,
        smart_switch: Arc<Mutex<minkey::smart_switch::SmartSwitchTable>>,
        tray_hwnd: usize,
    ) -> Self {
        Self {
            main_window: RefCell::new(None),
            macro_window: RefCell::new(None),
            convert_window: RefCell::new(None),
            app_config,
            engine,
            smart_switch,
            tray_hwnd,
        }
    }

    fn get_or_create_macro_window(&self) -> Option<MacroWindow> {
        let mut slot = self.macro_window.borrow_mut();
        if slot.is_none() {
            if let Ok(w) = MacroWindow::new() {
                w.set_opt_auto_caps(self.app_config.auto_caps_macro.load(Ordering::Relaxed));
                setup_macro_window_callbacks(&w, self.engine.clone(), self.app_config.clone());
                *slot = Some(w);
            }
        }
        slot.as_ref().map(|w| w.clone_strong())
    }

    fn get_or_create_convert_window(&self) -> Option<ConvertWindow> {
        let mut slot = self.convert_window.borrow_mut();
        if slot.is_none() {
            if let Ok(w) = ConvertWindow::new() {
                populate_convert_ui_from_config(&w, &self.app_config);
                setup_convert_window_callbacks(&w, self.app_config.clone());
                *slot = Some(w);
            }
        }
        slot.as_ref().map(|w| w.clone_strong())
    }

    fn get_or_create_main_window(&self) -> Option<MainWindow> {
        if let Some(w) = self.main_window.borrow().as_ref() {
            return Some(w.clone_strong());
        }
        let Ok(Ok(w)) = std::panic::catch_unwind(MainWindow::new) else {
            return None;
        };
        populate_ui_from_config(&w, &self.app_config);

        if let (Some(m_win), Some(c_win)) = (self.get_or_create_macro_window(), self.get_or_create_convert_window()) {
            setup_ui_callbacks(
                &w,
                self.app_config.clone(),
                self.engine.clone(),
                self.smart_switch.clone(),
                self.tray_hwnd,
                &m_win,
                &c_win,
            );
        }
        *self.main_window.borrow_mut() = Some(w.clone_strong());
        Some(w)
    }

    fn show_main_window(&self, active_tab: Option<i32>) {
        if let Some(ui) = self.get_or_create_main_window() {
            if let Some(tab) = active_tab {
                ui.set_active_tab(tab);
            }
            let _ = ui.show();
            #[cfg(target_os = "macos")]
            minkey::macos_app::activate();
        }
    }

    fn show_macro_window(&self) {
        if let Some(m_win) = self.get_or_create_macro_window() {
            if let Ok(eng) = self.engine.lock() {
                let filter = m_win.get_search_filter();
                update_macro_window_items(&m_win, &eng, filter.as_str());
            }
            let _ = m_win.show();
            #[cfg(target_os = "macos")]
            minkey::macos_app::activate();
        }
    }

    fn show_convert_window(&self) {
        if let Some(c_win) = self.get_or_create_convert_window() {
            populate_convert_ui_from_config(&c_win, &self.app_config);
            let _ = c_win.show();
            #[cfg(target_os = "macos")]
            minkey::macos_app::activate();
        }
    }
}

fn check_single_instance() -> bool {
    #[cfg(windows)]
    unsafe {
        let mutex_name = wide_null("Local\\MinkeyAppMutex");
        windows_sys::Win32::Foundation::SetLastError(0);
        let _mutex = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let tray_wnd = FindWindowW(wide_null("MinkeyTrayWndClass").as_ptr(), std::ptr::null());
            if !tray_wnd.is_null() {
                PostMessageW(tray_wnd, WM_USER + 1, 0, WM_LBUTTONDBLCLK as isize);
            }
            minkey::hook::MessageBeep(0);
            return false;
        }
    }
    true
}

fn main() {
    // Single instance guard
    if !check_single_instance() {
        return;
    }

    // Load application configuration (Registry on Windows, JSON file on macOS)
    let app_config = Arc::new(AppConfig::load_from_registry());

    // Create Vietnamese Engine singleton and sync settings
    let engine = Arc::new(Mutex::new(VietnameseEngine::new()));
    if let Ok(mut eng) = engine.lock() {
        sync_engine_from_config(&mut eng, &app_config);
    }

    // Load macro data
    if let Some(macro_bytes) = minkey::config::get_reg_binary("macroData") {
        if let Ok(eng) = engine.lock() {
            if let Ok(mut table) = eng.macro_table.lock() {
                *table = minkey::macro_engine::MacroTable::from_binary(&macro_bytes);
            }
        }
    }

    // Load smart switch key data
    let smart_switch = Arc::new(Mutex::new(
        if let Some(smart_bytes) = minkey::config::get_reg_binary("smartSwitchKey") {
            minkey::smart_switch::SmartSwitchTable::from_binary(&smart_bytes)
        } else {
            minkey::smart_switch::SmartSwitchTable::new()
        }
    ));

    let hook_config = app_config.to_hook_config();

    // Build System Tray callbacks (may run on any thread; UI work hops to the event loop)
    let tray_callbacks = TrayCallbacks {
        on_open_control_panel: Box::new(|| on_ui_thread(|wm| wm.show_main_window(None))),
        on_open_macro_table: Box::new(|| on_ui_thread(|wm| wm.show_macro_window())),
        on_open_convert_tool: Box::new(|| on_ui_thread(|wm| wm.show_convert_window())),
        on_open_about: Box::new(|| on_ui_thread(|wm| wm.show_main_window(Some(3)))),
        on_exit: Box::new(|| {
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        }),
    };

    // Initialize and start System Tray / Status Bar service
    let mut tray_service = PlatformTrayService::new(
        hook_config.clone(),
        engine.clone(),
        tray_callbacks,
    );
    tray_service.start(app_config.use_gray_icon.clone());
    let tray_hwnd = tray_service.get_hwnd();

    // Lazy UI Window Manager (Phương án 2: Zero UI windows instantiated at startup if idle)
    WINDOW_MANAGER.with(|cell| {
        let _ = cell.set(LazyWindowManager::new(
            app_config.clone(),
            engine.clone(),
            smart_switch.clone(),
            tray_hwnd,
        ));
    });

    // Initialize Low-Level Keyboard and Mouse Hook (Windows WH_KEYBOARD_LL / macOS CGEventTap)
    let mut hook_service = PlatformHookService::new(hook_config, engine.clone(), smart_switch.clone());
    let config_for_convert_hotkey = app_config.clone();
    let app_cfg_table = app_config.clone();

    hook_service.start(
        move |lang| {
            // Language toggled via hotkey
            minkey::tray::refresh_tray(tray_hwnd);
            on_ui_thread(move |wm| {
                if let Some(ui) = wm.main_window.borrow().as_ref() {
                    ui.set_is_vietnamese(lang != 0);
                }
            });
        },
        move |code_table| {
            // Code table changed via smart switch
            app_cfg_table.code_table.store(code_table, Ordering::Relaxed);
            minkey::tray::refresh_tray(tray_hwnd);
            on_ui_thread(move |wm| {
                if let Some(ui) = wm.main_window.borrow().as_ref() {
                    ui.set_selected_code_table(code_table as i32);
                }
            });
        },

        move || {
            // Quick convert clipboard hotkey (runs off the hook thread so typing never stalls)
            let config = config_for_convert_hotkey.clone();
            std::thread::spawn(move || {
                let opts = minkey::ui::get_convert_options_from_config(&config);
                if minkey::convert::quick_convert_clipboard(&opts) {
                    if !opts.dont_alert {
                        minkey::dialog::alert("Minkey - Chuyển mã", "Đã chuyển mã nội dung Clipboard thành công!");
                    } else {
                        minkey::hook::MessageBeep(0);
                    }
                }
            });
        },
    );

    // Beep to signal successful startup
    minkey::hook::MessageBeep(0);

    // Show control panel on startup only if configured by user
    if app_config.show_on_startup.load(Ordering::Relaxed) {
        with_wm(|wm| wm.show_main_window(None));
    } else {
        // Trim working set memory right away so background RAM starts at ~2.5 MB
        minkey::trim_process_memory();
    }

    // Run Slint event loop — blocks until quit_event_loop() is called
    let _ = slint::run_event_loop_until_quit();

    // Clean shutdown: unhook, stop tray, persist config
    hook_service.stop();
    tray_service.stop();
    app_config.save_to_registry();
    minkey::trim_process_memory();
}
