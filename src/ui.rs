// Modern Fluent Control Panel UI Controller
// Integrates Slint UI with Minkey Backend Engine and Config

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::config::{AppConfig, register_run_on_startup, DEFAULT_SWITCH_STATUS};
use crate::engine::VietnameseEngine;
use crate::types::InputType;

slint::include_modules!();

pub fn decode_hotkey(status: u32) -> (bool, bool, bool, bool, String, bool) {
    let ctrl = (status & 0x100) != 0;
    let alt = (status & 0x200) != 0;
    let win = (status & 0x400) != 0;
    let shift = (status & 0x800) != 0;
    let beep = (status & 0x8000) != 0;
    let key_code = (status & 0xFF) as u8;
    let key_char = if (0x20..=0x7E).contains(&key_code) {
        (key_code as char).to_ascii_uppercase().to_string()
    } else {
        "Z".to_string()
    };
    (ctrl, alt, win, shift, key_char, beep)
}

pub fn encode_hotkey(ctrl: bool, alt: bool, win: bool, shift: bool, key_str: &str, beep: bool) -> u32 {
    let mut status: u32 = 0;
    if ctrl { status |= 0x100; }
    if alt { status |= 0x200; }
    if win { status |= 0x400; }
    if shift { status |= 0x800; }
    if beep { status |= 0x8000; }

    let key_byte = key_str.chars().next().map(|c| c.to_ascii_uppercase() as u8).unwrap_or(b'Z');
    status |= key_byte as u32;
    status |= (key_byte as u32) << 24;
    status
}

pub fn populate_ui_from_config(ui: &MainWindow, config: &AppConfig) {
    ui.set_is_vietnamese(config.language.load(Ordering::Relaxed) != 0);
    ui.set_selected_input_type(config.input_type.load(Ordering::Relaxed) as i32);
    ui.set_selected_code_table(config.code_table.load(Ordering::Relaxed) as i32);

    let (ctrl, alt, win, shift, key, beep) = decode_hotkey(config.switch_key_status.load(Ordering::Relaxed));
    ui.set_switch_ctrl(ctrl);
    ui.set_switch_alt(alt);
    ui.set_switch_win(win);
    ui.set_switch_shift(shift);
    ui.set_switch_key(SharedString::from(key));
    ui.set_switch_beep(beep);

    // Tab 1
    ui.set_opt_modern_orthography(config.use_modern_orthography.load(Ordering::Relaxed));
    ui.set_opt_fix_browser(config.fix_recommend_browser.load(Ordering::Relaxed));
    ui.set_opt_check_spelling(config.check_spelling.load(Ordering::Relaxed));
    ui.set_opt_restore_spelling(config.restore_if_wrong_spelling.load(Ordering::Relaxed));
    ui.set_opt_allow_zfwj(config.allow_consonant_zfwj.load(Ordering::Relaxed));
    ui.set_opt_temp_off_spelling(config.temp_off_spelling.load(Ordering::Relaxed));
    ui.set_opt_smart_switch(config.use_smart_switch_key.load(Ordering::Relaxed));
    ui.set_opt_upper_first(config.upper_case_first_char.load(Ordering::Relaxed));
    ui.set_opt_remember_code(config.remember_code.load(Ordering::Relaxed));
    ui.set_opt_other_language(config.other_language.load(Ordering::Relaxed));
    ui.set_opt_temp_off_openkey(config.temp_off_openkey.load(Ordering::Relaxed));

    // Tab 2
    ui.set_opt_quick_telex(config.quick_telex.load(Ordering::Relaxed));
    ui.set_opt_use_macro(config.use_macro.load(Ordering::Relaxed));
    ui.set_opt_macro_in_english(config.use_macro_in_english.load(Ordering::Relaxed));
    ui.set_opt_macro_auto_caps(config.auto_caps_macro.load(Ordering::Relaxed));
    ui.set_opt_quick_start_consonant(config.quick_start_consonant.load(Ordering::Relaxed));
    ui.set_opt_quick_end_consonant(config.quick_end_consonant.load(Ordering::Relaxed));

    // Tab 3
    ui.set_opt_modern_icon(config.use_gray_icon.load(Ordering::Relaxed));
    ui.set_opt_show_on_startup(config.show_on_startup.load(Ordering::Relaxed));
    ui.set_opt_run_with_windows(config.run_with_windows.load(Ordering::Relaxed));
    ui.set_opt_run_as_admin(config.run_as_admin.load(Ordering::Relaxed));
    ui.set_opt_check_update(config.check_new_version.load(Ordering::Relaxed));
    ui.set_opt_desktop_shortcut(config.create_desktop_shortcut.load(Ordering::Relaxed));
    ui.set_opt_fix_chromium(config.fix_chromium_browser.load(Ordering::Relaxed));
    ui.set_opt_use_clipboard(!config.send_key_step_by_step.load(Ordering::Relaxed));
    ui.set_opt_support_metro(config.support_metro_app.load(Ordering::Relaxed));
}

pub fn setup_ui_callbacks(
    ui: &MainWindow,
    config: Arc<AppConfig>,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    tray_hwnd: usize,
    macro_win: &MacroWindow,
    convert_win: &ConvertWindow,
) {
    let ui_weak = ui.as_weak();

    // Language toggle
    {
        let config = config.clone();
        let engine = engine.clone();
        let smart_switch = smart_switch.clone();
        ui.on_language_changed(move |is_viet| {
            let lang = if is_viet { 1 } else { 0 };
            config.language.store(lang, Ordering::SeqCst);
            if let Ok(mut eng) = engine.lock() {
                eng.language = lang;
                eng.start_new_session();
            }
            if config.use_smart_switch_key.load(Ordering::Relaxed) {
                if let Some(app) = crate::smart_switch::get_frontmost_app_name() {
                    if let Ok(mut table) = smart_switch.lock() {
                        let cur_table = config.code_table.load(Ordering::Relaxed) as usize;
                        table.update(&app, lang, cur_table);
                        let bin = table.to_binary();
                        crate::config::set_reg_binary("smartSwitchKey", &bin);
                    }
                }
            }
            crate::tray::refresh_tray(tray_hwnd);
            config.save_to_registry();
        });
    }

    // Input Type Changed (Telex, VNI, Simple Telex 1, Simple Telex 2)
    {
        let config = config.clone();
        let engine = engine.clone();
        ui.on_input_type_changed(move |idx| {
            config.input_type.store(idx as u32, Ordering::SeqCst);
            if let Ok(mut eng) = engine.lock() {
                eng.input_type = match idx {
                    0 => InputType::Telex,
                    1 => InputType::Vni,
                    2 => InputType::SimpleTelex1,
                    3 => InputType::SimpleTelex2,
                    _ => InputType::Telex,
                };
                eng.start_new_session();
            }
            config.save_to_registry();
        });
    }

    // Code Table Changed
    {
        let config = config.clone();
        let engine = engine.clone();
        let smart_switch = smart_switch.clone();
        ui.on_code_table_changed(move |idx| {
            config.code_table.store(idx as u32, Ordering::SeqCst);
            if let Ok(mut eng) = engine.lock() {
                eng.code_table = idx as usize;
            }
            if config.remember_code.load(Ordering::Relaxed) {
                if let Some(app) = crate::smart_switch::get_frontmost_app_name() {
                    if let Ok(mut table) = smart_switch.lock() {
                        let cur_lang = config.language.load(Ordering::Relaxed);
                        table.update(&app, cur_lang, idx as usize);
                        let bin = table.to_binary();
                        crate::config::set_reg_binary("smartSwitchKey", &bin);
                    }
                }
            }
            config.save_to_registry();
        });
    }

    // Switch Hotkey Changed
    {
        let config = config.clone();
        let ui_weak = ui_weak.clone();
        ui.on_switch_hotkey_changed(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let ctrl = ui.get_switch_ctrl();
                let alt = ui.get_switch_alt();
                let win = ui.get_switch_win();
                let shift = ui.get_switch_shift();
                let key_str = ui.get_switch_key();
                let beep = ui.get_switch_beep();

                let code = encode_hotkey(ctrl, alt, win, shift, &key_str, beep);
                config.switch_key_status.store(code, Ordering::SeqCst);
                config.save_to_registry();
            }
        });
    }

    // Individual Setting Toggles
    {
        let config = config.clone();
        let engine = engine.clone();
        ui.on_setting_toggled(move |name, val| {
            let key = name.as_str();
            match key {
                "modern_orthography" => {
                    config.use_modern_orthography.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.use_modern_orthography = val; }
                }
                "fix_browser" => {
                    config.fix_recommend_browser.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.fix_recommend_browser = val; }
                }
                "check_spelling" => {
                    config.check_spelling.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.set_check_spelling(val); }
                }
                "restore_spelling" => {
                    config.restore_if_wrong_spelling.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.restore_if_wrong_spelling = val; }
                }
                "allow_zfwj" => {
                    config.allow_consonant_zfwj.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.allow_consonant_zfwj = val; }
                }
                "temp_off_spelling" => {
                    config.temp_off_spelling.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.temp_off_spelling = val; }
                }
                "smart_switch" => {
                    config.use_smart_switch_key.store(val, Ordering::Relaxed);
                }
                "upper_first" => {
                    config.upper_case_first_char.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.upper_case_first_char = val; }
                }
                "remember_code" => {
                    config.remember_code.store(val, Ordering::Relaxed);
                }
                "other_language" => {
                    config.other_language.store(val, Ordering::Relaxed);
                }
                "temp_off_openkey" => {
                    config.temp_off_openkey.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.temp_off_openkey = val; }
                }
                "quick_telex" => {
                    config.quick_telex.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.quick_telex = val; }
                }
                "use_macro" => {
                    config.use_macro.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.use_macro = val; }
                }
                "macro_in_english" => {
                    config.use_macro_in_english.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.use_macro_in_english = val; }
                }
                "macro_auto_caps" => {
                    config.auto_caps_macro.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.auto_caps_macro = val; }
                }
                "quick_start_consonant" => {
                    config.quick_start_consonant.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.quick_start_consonant = val; }
                }
                "quick_end_consonant" => {
                    config.quick_end_consonant.store(val, Ordering::Relaxed);
                    if let Ok(mut eng) = engine.lock() { eng.quick_end_consonant = val; }
                }
                "modern_icon" => {
                    config.use_gray_icon.store(val, Ordering::Relaxed);
                    crate::tray::refresh_tray(tray_hwnd);
                }
                "show_on_startup" => {
                    config.show_on_startup.store(val, Ordering::Relaxed);
                }
                "run_with_windows" => {
                    config.run_with_windows.store(val, Ordering::Relaxed);
                    register_run_on_startup(val, config.run_as_admin.load(Ordering::Relaxed));
                }
                "run_as_admin" => {
                    config.run_as_admin.store(val, Ordering::Relaxed);
                    if val && !crate::config::is_user_an_admin() {
                        if crate::dialog::confirm(
                            "Minkey",
                            "Bạn cần phải khởi động lại Minkey để kích hoạt chế độ Admin!\nBạn có muốn khởi động lại Minkey không?",
                        ) {
                            crate::config::relaunch_as_admin();
                            let _ = slint::invoke_from_event_loop(|| {
                                let _ = slint::quit_event_loop();
                            });
                        }
                    } else {
                        register_run_on_startup(config.run_with_windows.load(Ordering::Relaxed), val);
                    }
                }
                "check_update" => {
                    config.check_new_version.store(val, Ordering::Relaxed);
                }
                "desktop_shortcut" => {
                    config.create_desktop_shortcut.store(val, Ordering::Relaxed);
                    if val {
                        crate::config::create_desktop_shortcut();
                    }
                }
                "fix_chromium" => {
                    config.fix_chromium_browser.store(val, Ordering::Relaxed);
                }
                "use_clipboard" => {
                    config.send_key_step_by_step.store(!val, Ordering::Relaxed);
                }
                "support_metro" => {
                    config.support_metro_app.store(val, Ordering::Relaxed);
                }
                _ => {}
            }
            config.save_to_registry();
        });
    }

    // Reset Defaults
    {
        let config = config.clone();
        let engine = engine.clone();
        let ui_weak = ui_weak.clone();
        ui.on_reset_defaults(move || {
            if !crate::dialog::confirm("Minkey", "Bạn có chắc chắn muốn thiết lập lại cài đặt gốc?") {
                return;
            }

            let def = AppConfig::default();
            config.language.store(def.language.load(Ordering::Relaxed), Ordering::Relaxed);
            config.input_type.store(def.input_type.load(Ordering::Relaxed), Ordering::Relaxed);
            config.code_table.store(def.code_table.load(Ordering::Relaxed), Ordering::Relaxed);
            config.check_spelling.store(def.check_spelling.load(Ordering::Relaxed), Ordering::Relaxed);
            config.use_modern_orthography.store(def.use_modern_orthography.load(Ordering::Relaxed), Ordering::Relaxed);
            config.quick_telex.store(def.quick_telex.load(Ordering::Relaxed), Ordering::Relaxed);
            config.switch_key_status.store(DEFAULT_SWITCH_STATUS, Ordering::Relaxed);
            config.restore_if_wrong_spelling.store(def.restore_if_wrong_spelling.load(Ordering::Relaxed), Ordering::Relaxed);
            config.fix_recommend_browser.store(def.fix_recommend_browser.load(Ordering::Relaxed), Ordering::Relaxed);
            config.use_macro.store(def.use_macro.load(Ordering::Relaxed), Ordering::Relaxed);
            config.use_macro_in_english.store(def.use_macro_in_english.load(Ordering::Relaxed), Ordering::Relaxed);
            config.auto_caps_macro.store(def.auto_caps_macro.load(Ordering::Relaxed), Ordering::Relaxed);
            config.send_key_step_by_step.store(def.send_key_step_by_step.load(Ordering::Relaxed), Ordering::Relaxed);
            config.use_gray_icon.store(def.use_gray_icon.load(Ordering::Relaxed), Ordering::Relaxed);
            config.show_on_startup.store(def.show_on_startup.load(Ordering::Relaxed), Ordering::Relaxed);
            config.run_with_windows.store(def.run_with_windows.load(Ordering::Relaxed), Ordering::Relaxed);
            config.use_smart_switch_key.store(def.use_smart_switch_key.load(Ordering::Relaxed), Ordering::Relaxed);
            config.upper_case_first_char.store(def.upper_case_first_char.load(Ordering::Relaxed), Ordering::Relaxed);
            config.allow_consonant_zfwj.store(def.allow_consonant_zfwj.load(Ordering::Relaxed), Ordering::Relaxed);
            config.temp_off_spelling.store(def.temp_off_spelling.load(Ordering::Relaxed), Ordering::Relaxed);
            config.quick_start_consonant.store(def.quick_start_consonant.load(Ordering::Relaxed), Ordering::Relaxed);
            config.quick_end_consonant.store(def.quick_end_consonant.load(Ordering::Relaxed), Ordering::Relaxed);
            config.support_metro_app.store(def.support_metro_app.load(Ordering::Relaxed), Ordering::Relaxed);
            config.run_as_admin.store(def.run_as_admin.load(Ordering::Relaxed), Ordering::Relaxed);
            config.create_desktop_shortcut.store(def.create_desktop_shortcut.load(Ordering::Relaxed), Ordering::Relaxed);
            config.check_new_version.store(def.check_new_version.load(Ordering::Relaxed), Ordering::Relaxed);
            config.remember_code.store(def.remember_code.load(Ordering::Relaxed), Ordering::Relaxed);
            config.other_language.store(def.other_language.load(Ordering::Relaxed), Ordering::Relaxed);
            config.temp_off_openkey.store(def.temp_off_openkey.load(Ordering::Relaxed), Ordering::Relaxed);
            config.fix_chromium_browser.store(def.fix_chromium_browser.load(Ordering::Relaxed), Ordering::Relaxed);

            if let Ok(mut eng) = engine.lock() {
                eng.input_type = InputType::Telex;
                eng.code_table = 0;
                eng.check_spelling = true;
                eng.use_modern_orthography = false;
                eng.quick_telex = false;
                eng.restore_if_wrong_spelling = true;
                eng.fix_recommend_browser = true;
                eng.use_macro = true;
                eng.use_macro_in_english = false;
                eng.auto_caps_macro = false;
                eng.upper_case_first_char = false;
                eng.allow_consonant_zfwj = false;
                eng.quick_start_consonant = false;
                eng.quick_end_consonant = false;
                eng.temp_off_spelling = false;
                eng.temp_off_openkey = false;
                eng.start_new_session();
            }

            config.save_to_registry();

            if let Some(ui) = ui_weak.upgrade() {
                populate_ui_from_config(&ui, &config);
            }
        });
    }

    // Open GitHub
    ui.on_open_github(|| {
        crate::dialog::open_url("https://github.com/tuyenvm/OpenKey");
    });

    // Check Update
    ui.on_check_update_clicked(|| {
        crate::dialog::alert("Minkey - Cập nhật", "Bạn đang sử dụng phiên bản mới nhất! (Minkey v0.1.0)");
    });

    // Close window (Hide to tray)
    {
        let ui_weak = ui_weak.clone();
        ui.on_close_window(move || {
            if let Some(ui) = ui_weak.upgrade() {
                let _ = ui.hide();
            }
        });
    }

    // Open Macro Table
    {
        let macro_win_weak = macro_win.as_weak();
        let engine = engine.clone();
        ui.on_open_macro_table(move || {
            if let Some(m_win) = macro_win_weak.upgrade() {
                if let Ok(eng) = engine.lock() {
                    let filter = m_win.get_search_filter();
                    update_macro_window_items(&m_win, &eng, filter.as_str());
                }
                let _ = m_win.show();
            }
        });
    }

    // Open Convert Tool
    {
        let convert_win_weak = convert_win.as_weak();
        let config = config.clone();
        ui.on_open_convert_tool(move || {
            if let Some(c_win) = convert_win_weak.upgrade() {
                populate_convert_ui_from_config(&c_win, &config);
                let _ = c_win.show();
            }
        });
    }

    // Exit app completely
    ui.on_exit_app(|| {
        let _ = slint::quit_event_loop();
    });
}

pub fn update_macro_window_items(macro_win: &MacroWindow, engine: &VietnameseEngine, filter: &str) {
    let filter_lower = filter.trim().to_lowercase();
    let all = if let Ok(table) = engine.macro_table.lock() {
        table.get_all()
    } else {
        Vec::new()
    };
    let items: Vec<MacroItem> = all
        .into_iter()
        .filter(|m| {
            if filter_lower.is_empty() {
                true
            } else {
                m.text.to_lowercase().contains(&filter_lower)
                    || m.content.to_lowercase().contains(&filter_lower)
            }
        })
        .map(|m| MacroItem {
            text: SharedString::from(m.text),
            content: SharedString::from(m.content),
        })
        .collect();
    macro_win.set_items(ModelRc::new(VecModel::from(items)));
}

pub fn setup_macro_window_callbacks(
    macro_win: &MacroWindow,
    engine: Arc<Mutex<VietnameseEngine>>,
    config: Arc<AppConfig>,
) {
    let win_weak = macro_win.as_weak();

    // AutoCaps toggle
    {
        let config = config.clone();
        let engine = engine.clone();
        macro_win.on_autocaps_toggled(move |val| {
            config.auto_caps_macro.store(val, Ordering::Relaxed);
            if let Ok(mut eng) = engine.lock() {
                eng.auto_caps_macro = val;
            }
            config.save_to_registry();
        });
    }

    // Input text changed -> auto-fill existing replacement if found
    {
        let win_weak = win_weak.clone();
        let engine = engine.clone();
        macro_win.on_input_text_changed(move |text| {
            let key = text.as_str().trim();
            if let Ok(eng) = engine.lock() {
                if let Ok(table) = eng.macro_table.lock() {
                    if let Some(content) = table.map.get(key) {
                        if let Some(win) = win_weak.upgrade() {
                            win.set_input_content(SharedString::from(content));
                            win.set_is_editing(true);
                        }
                    }
                }
            }
        });
    }

    // Search filter changed
    {
        let win_weak = win_weak.clone();
        let engine = engine.clone();
        macro_win.on_search_filter_changed(move |filter| {
            if let Some(win) = win_weak.upgrade() {
                if let Ok(eng) = engine.lock() {
                    update_macro_window_items(&win, &eng, filter.as_str());
                }
            }
        });
    }

    // Add or update macro
    {
        let win_weak = win_weak.clone();
        let engine = engine.clone();
        macro_win.on_add_or_update_macro(move |text, content| {
            let key = text.as_str().trim();
            let val = content.as_str().trim();
            if key.is_empty() || val.is_empty() {
                return;
            }

            if let Ok(eng) = engine.lock() {
                if let Ok(mut table) = eng.macro_table.lock() {
                    table.add(key, val);
                    let bin = table.to_binary();
                    crate::config::set_reg_binary("macroData", &bin);
                }

                if let Some(win) = win_weak.upgrade() {
                    let filter = win.get_search_filter();
                    update_macro_window_items(&win, &eng, filter.as_str());
                }
            }
        });
    }

    // Delete macro
    {
        let win_weak = win_weak.clone();
        let engine = engine.clone();
        macro_win.on_delete_macro(move |text| {
            let key = text.as_str().trim();
            if key.is_empty() {
                return;
            }

            if let Ok(eng) = engine.lock() {
                let deleted = if let Ok(mut table) = eng.macro_table.lock() {
                    if table.delete(key) {
                        let bin = table.to_binary();
                        crate::config::set_reg_binary("macroData", &bin);
                        true
                    } else {
                        false
                    }
                } else {
                    false
                };

                if deleted {
                    if let Some(win) = win_weak.upgrade() {
                        let filter = win.get_search_filter();
                        update_macro_window_items(&win, &eng, filter.as_str());
                    }
                }
            }
        });
    }

    // Import from .txt file
    {
        let win_weak = win_weak.clone();
        let engine = engine.clone();
        macro_win.on_import_clicked(move || {
            if let Some(path) = crate::macro_engine::open_macro_file_dialog() {
                let keep_existing = crate::macro_engine::ask_keep_existing_macros();
                if let Ok(eng) = engine.lock() {
                    if let Ok(mut table) = eng.macro_table.lock() {
                        let _ = table.import_file(&path, keep_existing);
                        let bin = table.to_binary();
                        crate::config::set_reg_binary("macroData", &bin);
                    }

                    if let Some(win) = win_weak.upgrade() {
                        let filter = win.get_search_filter();
                        update_macro_window_items(&win, &eng, filter.as_str());
                    }
                }
            }
        });
    }

    // Export to .txt file
    {
        let engine = engine.clone();
        macro_win.on_export_clicked(move || {
            if let Some(path) = crate::macro_engine::save_macro_file_dialog() {
                if let Ok(eng) = engine.lock() {
                    if let Ok(table) = eng.macro_table.lock() {
                        let _ = table.export_file(&path);
                    }
                }
            }
        });
    }

    // Close window
    {
        let win_weak = win_weak.clone();
        macro_win.on_close_window(move || {
            if let Some(win) = win_weak.upgrade() {
                let _ = win.hide();
            }
        });
    }
}

pub fn populate_convert_ui_from_config(ui: &ConvertWindow, config: &AppConfig) {
    ui.set_selected_from_code(config.convert_from_code.load(Ordering::Relaxed) as i32);
    ui.set_selected_to_code(config.convert_to_code.load(Ordering::Relaxed) as i32);

    ui.set_opt_to_all_caps(config.convert_to_all_caps.load(Ordering::Relaxed));
    ui.set_opt_to_all_non_caps(config.convert_to_all_non_caps.load(Ordering::Relaxed));
    ui.set_opt_to_caps_each_word(config.convert_to_caps_each_word.load(Ordering::Relaxed));
    ui.set_opt_to_caps_first_letter(config.convert_to_caps_first_letter.load(Ordering::Relaxed));
    ui.set_opt_remove_mark(config.convert_remove_mark.load(Ordering::Relaxed));
    ui.set_opt_dont_alert(config.convert_dont_alert.load(Ordering::Relaxed));

    let (ctrl, alt, win, shift, key, _) = decode_hotkey(config.convert_hotkey.load(Ordering::Relaxed));
    ui.set_switch_ctrl(ctrl);
    ui.set_switch_alt(alt);
    ui.set_switch_win(win);
    ui.set_switch_shift(shift);
    ui.set_switch_key(SharedString::from(key));
}

pub fn get_convert_options_from_config(config: &AppConfig) -> crate::convert::ConvertOptions {
    crate::convert::ConvertOptions {
        from_code: config.convert_from_code.load(Ordering::Relaxed) as usize,
        to_code: config.convert_to_code.load(Ordering::Relaxed) as usize,
        to_all_caps: config.convert_to_all_caps.load(Ordering::Relaxed),
        to_all_non_caps: config.convert_to_all_non_caps.load(Ordering::Relaxed),
        to_caps_first_letter: config.convert_to_caps_first_letter.load(Ordering::Relaxed),
        to_caps_each_word: config.convert_to_caps_each_word.load(Ordering::Relaxed),
        remove_mark: config.convert_remove_mark.load(Ordering::Relaxed),
        dont_alert: config.convert_dont_alert.load(Ordering::Relaxed),
    }
}

pub fn setup_convert_window_callbacks(
    convert_win: &ConvertWindow,
    config: Arc<AppConfig>,
) {
    let win_weak = convert_win.as_weak();

    // Swap Codes button
    {
        let win_weak = win_weak.clone();
        let config = config.clone();
        convert_win.on_swap_codes(move || {
            if let Some(win) = win_weak.upgrade() {
                let from = win.get_selected_from_code();
                let to = win.get_selected_to_code();
                win.set_selected_from_code(to);
                win.set_selected_to_code(from);
                config.convert_from_code.store(to as u32, Ordering::Relaxed);
                config.convert_to_code.store(from as u32, Ordering::Relaxed);
                config.save_to_registry();
            }
        });
    }

    // Code table dropdown changed
    {
        let win_weak = win_weak.clone();
        let config = config.clone();
        convert_win.on_codes_changed(move || {
            if let Some(win) = win_weak.upgrade() {
                config.convert_from_code.store(win.get_selected_from_code() as u32, Ordering::Relaxed);
                config.convert_to_code.store(win.get_selected_to_code() as u32, Ordering::Relaxed);
                config.save_to_registry();
            }
        });
    }

    // Setting toggled with mutual exclusion matching OpenKey setLogic
    {
        let win_weak = win_weak.clone();
        let config = config.clone();
        convert_win.on_setting_changed(move |name, val| {
            if let Some(win) = win_weak.upgrade() {
                match name.as_str() {
                    "to_all_caps" => {
                        config.convert_to_all_caps.store(val, Ordering::Relaxed);
                        if val {
                            win.set_opt_to_all_non_caps(false);
                            win.set_opt_to_caps_each_word(false);
                            win.set_opt_to_caps_first_letter(false);
                            config.convert_to_all_non_caps.store(false, Ordering::Relaxed);
                            config.convert_to_caps_each_word.store(false, Ordering::Relaxed);
                            config.convert_to_caps_first_letter.store(false, Ordering::Relaxed);
                        }
                    }
                    "to_all_non_caps" => {
                        config.convert_to_all_non_caps.store(val, Ordering::Relaxed);
                        if val {
                            win.set_opt_to_all_caps(false);
                            win.set_opt_to_caps_each_word(false);
                            win.set_opt_to_caps_first_letter(false);
                            config.convert_to_all_caps.store(false, Ordering::Relaxed);
                            config.convert_to_caps_each_word.store(false, Ordering::Relaxed);
                            config.convert_to_caps_first_letter.store(false, Ordering::Relaxed);
                        }
                    }
                    "to_caps_each_word" => {
                        config.convert_to_caps_each_word.store(val, Ordering::Relaxed);
                        if val {
                            win.set_opt_to_all_caps(false);
                            win.set_opt_to_all_non_caps(false);
                            win.set_opt_to_caps_first_letter(false);
                            config.convert_to_all_caps.store(false, Ordering::Relaxed);
                            config.convert_to_all_non_caps.store(false, Ordering::Relaxed);
                            config.convert_to_caps_first_letter.store(false, Ordering::Relaxed);
                        }
                    }
                    "to_caps_first_letter" => {
                        config.convert_to_caps_first_letter.store(val, Ordering::Relaxed);
                        if val {
                            win.set_opt_to_all_caps(false);
                            win.set_opt_to_all_non_caps(false);
                            win.set_opt_to_caps_each_word(false);
                            config.convert_to_all_caps.store(false, Ordering::Relaxed);
                            config.convert_to_all_non_caps.store(false, Ordering::Relaxed);
                            config.convert_to_caps_each_word.store(false, Ordering::Relaxed);
                        }
                    }
                    "remove_mark" => {
                        config.convert_remove_mark.store(val, Ordering::Relaxed);
                    }
                    "dont_alert" => {
                        config.convert_dont_alert.store(val, Ordering::Relaxed);
                    }
                    _ => {}
                }
                config.save_to_registry();
            }
        });
    }

    // Hotkey changed
    {
        let win_weak = win_weak.clone();
        let config = config.clone();
        convert_win.on_hotkey_changed(move || {
            if let Some(win) = win_weak.upgrade() {
                let ctrl = win.get_switch_ctrl();
                let alt = win.get_switch_alt();
                let win_key = win.get_switch_win();
                let shift = win.get_switch_shift();
                let key_str = win.get_switch_key();

                let code = encode_hotkey(ctrl, alt, win_key, shift, &key_str, false);
                config.convert_hotkey.store(code, Ordering::SeqCst);
                config.save_to_registry();
            }
        });
    }

    // Convert Clicked
    {
        let config = config.clone();
        convert_win.on_convert_clicked(move || {
            let opts = get_convert_options_from_config(&config);
            let ok = crate::convert::quick_convert_clipboard(&opts);
            if ok {
                if !opts.dont_alert {
                    crate::dialog::alert("Minkey - Chuyển mã", "Đã chuyển mã nội dung Clipboard thành công!");
                } else {
                    crate::hook::MessageBeep(0);
                }
            }
        });
    }

    // Close window
    {
        let win_weak = win_weak.clone();
        convert_win.on_close_window(move || {
            if let Some(win) = win_weak.upgrade() {
                let _ = win.hide();
            }
        });
    }
}


