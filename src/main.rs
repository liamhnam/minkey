// Minkey - Modern Open-Source Vietnamese Input Method (Rust Port of OpenKey)
// Entry point and service coordinator

#![windows_subsystem = "windows"]

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use slint::ComponentHandle;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

use minkey::config::AppConfig;
use minkey::engine::VietnameseEngine;
use minkey::hook::WindowsHookService;
use minkey::tray::{TrayCallbacks, TrayService};
use minkey::types::InputType;
use minkey::ui::{
    populate_convert_ui_from_config, populate_ui_from_config, setup_convert_window_callbacks,
    setup_macro_window_callbacks, setup_ui_callbacks, update_macro_window_items,
    ConvertWindow, MainWindow, MacroWindow,
};

#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn CreateMutexW(
        lp_mutex_attributes: *const std::ffi::c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> HANDLE;
}

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

fn main() {
    // Single instance guard — matching OpenKey behavior
    let mutex_name = wide_null("Local\\MinkeyAppMutex");
    unsafe { windows_sys::Win32::Foundation::SetLastError(0) };
    let _mutex = unsafe { CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr()) };
    let last_err = unsafe { GetLastError() };
    if last_err == ERROR_ALREADY_EXISTS {
        // Another instance is running — show its window and exit
        unsafe {
            let tray_wnd = FindWindowW(wide_null("MinkeyTrayWndClass").as_ptr(), std::ptr::null());
            if !tray_wnd.is_null() {
                PostMessageW(tray_wnd, WM_USER + 1, 0, WM_LBUTTONDBLCLK as isize);
            }
            minkey::hook::MessageBeep(0);
        }
        return;
    }

    // Load application configuration from registry
    let app_config = Arc::new(AppConfig::load_from_registry());

    // Create Vietnamese Engine singleton and sync settings
    let engine = Arc::new(Mutex::new(VietnameseEngine::new()));
    if let Ok(mut eng) = engine.lock() {
        sync_engine_from_config(&mut eng, &app_config);
    }

    // Load macro data from registry
    if let Some(macro_bytes) = minkey::config::get_reg_binary("macroData") {
        if let Ok(eng) = engine.lock() {
            if let Ok(mut table) = eng.macro_table.lock() {
                *table = minkey::macro_engine::MacroTable::from_binary(&macro_bytes);
            }
        }
    }

    // Load smart switch key data from registry
    let smart_switch = Arc::new(Mutex::new(
        if let Some(smart_bytes) = minkey::config::get_reg_binary("smartSwitchKey") {
            minkey::smart_switch::SmartSwitchTable::from_binary(&smart_bytes)
        } else {
            minkey::smart_switch::SmartSwitchTable::new()
        }
    ));

    let hook_config = app_config.to_hook_config();

    // Create Slint UI windows — catch_unwind guards against renderer failures
    let main_window = match std::panic::catch_unwind(MainWindow::new) {
        Ok(Ok(w)) => w,
        Ok(Err(_)) | Err(_) => return,
    };
    populate_ui_from_config(&main_window, &app_config);

    let macro_window = match MacroWindow::new() {
        Ok(w) => w,
        Err(_) => return,
    };
    macro_window.set_opt_auto_caps(app_config.auto_caps_macro.load(Ordering::Relaxed));
    setup_macro_window_callbacks(&macro_window, engine.clone(), app_config.clone());

    let convert_window = match ConvertWindow::new() {
        Ok(w) => w,
        Err(_) => return,
    };
    populate_convert_ui_from_config(&convert_window, &app_config);
    setup_convert_window_callbacks(&convert_window, app_config.clone());

    // Build System Tray callbacks
    let ui_weak_for_tray = main_window.as_weak();
    let macro_win_weak_for_tray = macro_window.as_weak();
    let convert_win_weak_for_tray = convert_window.as_weak();
    let engine_for_tray_macro = engine.clone();
    let config_for_tray_convert = app_config.clone();

    let tray_callbacks = TrayCallbacks {
        on_open_control_panel: Box::new({
            let ui_weak = ui_weak_for_tray.clone();
            move || {
                let _ = slint::invoke_from_event_loop({
                    let ui_weak = ui_weak.clone();
                    move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            let _ = ui.show();
                        }
                    }
                });
            }
        }),
        on_open_macro_table: Box::new({
            let m_weak = macro_win_weak_for_tray.clone();
            let eng_clone = engine_for_tray_macro.clone();
            move || {
                let _ = slint::invoke_from_event_loop({
                    let m_weak = m_weak.clone();
                    let eng_clone = eng_clone.clone();
                    move || {
                        if let Some(m_win) = m_weak.upgrade() {
                            if let Ok(eng) = eng_clone.lock() {
                                let filter = m_win.get_search_filter();
                                update_macro_window_items(&m_win, &eng, filter.as_str());
                            }
                            let _ = m_win.show();
                        }
                    }
                });
            }
        }),
        on_open_convert_tool: Box::new({
            let c_weak = convert_win_weak_for_tray.clone();
            let cfg_clone = config_for_tray_convert.clone();
            move || {
                let _ = slint::invoke_from_event_loop({
                    let c_weak = c_weak.clone();
                    let cfg_clone = cfg_clone.clone();
                    move || {
                        if let Some(c_win) = c_weak.upgrade() {
                            populate_convert_ui_from_config(&c_win, &cfg_clone);
                            let _ = c_win.show();
                        }
                    }
                });
            }
        }),
        on_open_about: Box::new({
            let ui_weak = ui_weak_for_tray.clone();
            move || {
                let _ = slint::invoke_from_event_loop({
                    let ui_weak = ui_weak.clone();
                    move || {
                        if let Some(ui) = ui_weak.upgrade() {
                            ui.set_active_tab(3);
                            let _ = ui.show();
                        }
                    }
                });
            }
        }),
        on_exit: Box::new(|| {
            let _ = slint::invoke_from_event_loop(|| {
                let _ = slint::quit_event_loop();
            });
        }),
    };

    // Initialize and start System Tray service
    let mut tray_service = TrayService::new(
        hook_config.clone(),
        engine.clone(),
        tray_callbacks,
    );
    tray_service.start(app_config.use_gray_icon.clone());
    let tray_hwnd = tray_service.get_hwnd();

    // Setup Slint UI callbacks (settings panels, language/table pickers, etc.)
    setup_ui_callbacks(
        &main_window,
        app_config.clone(),
        engine.clone(),
        smart_switch.clone(),
        tray_hwnd,
        &macro_window,
        &convert_window,
    );

    // Initialize Low-Level Keyboard and Mouse Hook
    let mut hook_service = WindowsHookService::new(hook_config, engine.clone(), smart_switch.clone());
    let ui_weak_for_hook = main_window.as_weak();
    let config_for_convert_hotkey = app_config.clone();
    let ui_weak_table = main_window.as_weak();
    let app_cfg_table = app_config.clone();

    hook_service.start(
        move |lang| {
            // Language toggled via hotkey (Alt+Z or Ctrl+Shift)
            if tray_hwnd != 0 {
                unsafe {
                    PostMessageW(tray_hwnd as HWND, WM_USER + 2026, 0, 0);
                }
            }
            let ui_weak = ui_weak_for_hook.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_is_vietnamese(lang != 0);
                }
            });
        },
        move |code_table| {
            // Code table changed via smart switch
            app_cfg_table.code_table.store(code_table, Ordering::Relaxed);
            let ui_weak = ui_weak_table.clone();
            let _ = slint::invoke_from_event_loop(move || {
                if let Some(ui) = ui_weak.upgrade() {
                    ui.set_selected_code_table(code_table as i32);
                }
            });
        },
        move || {
            // Quick convert clipboard hotkey
            let opts = minkey::ui::get_convert_options_from_config(&config_for_convert_hotkey);
            let ok = minkey::convert::quick_convert_clipboard(&opts);
            if ok {
                if !opts.dont_alert {
                    unsafe {
                        let text: Vec<u16> = "Đã chuyển mã nội dung Clipboard thành công!\0".encode_utf16().collect();
                        let title: Vec<u16> = "Minkey - Chuyển mã\0".encode_utf16().collect();
                        MessageBoxW(
                            std::ptr::null_mut(),
                            text.as_ptr(),
                            title.as_ptr(),
                            MB_ICONINFORMATION | MB_OK,
                        );
                    }
                } else {
                    unsafe { minkey::hook::MessageBeep(0) };
                }
            }
        },
    );

    // Beep to signal successful startup
    unsafe { minkey::hook::MessageBeep(0) };

    // Show control panel on startup if configured
    if app_config.show_on_startup.load(Ordering::Relaxed) {
        let _ = main_window.show();
    }

    // Run Slint event loop — blocks until quit_event_loop() is called (Exit menu)
    let _ = slint::run_event_loop_until_quit();

    // Clean shutdown: unhook, stop tray, persist config
    hook_service.stop();
    tray_service.stop();
    app_config.save_to_registry();
}
