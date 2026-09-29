// Operations behind the control panel: each one updates AppConfig, the running engine and the
// saved settings together, so the UI code only reads and writes widgets.

use std::sync::atomic::Ordering;

use crate::app::AppContext;
use crate::config::{AppConfig, DEFAULT_SWITCH_STATUS, EMPTY_HOTKEY, register_run_on_startup};
use crate::convert::ConvertOptions;
use crate::engine::VietnameseEngine;
use crate::macro_engine::{MacroEntry, MacroTable};
use crate::types::InputType;

pub fn sync_engine_from_config(engine: &mut VietnameseEngine, config: &AppConfig) {
    engine.language = config.language.load(Ordering::Relaxed);
    engine.input_type = InputType::from_u32(config.input_type.load(Ordering::Relaxed));
    engine.code_table = config.code_table.load(Ordering::Relaxed) as usize;
    engine.set_check_spelling(config.check_spelling.load(Ordering::Relaxed));
    engine.use_modern_orthography = config.use_modern_orthography.load(Ordering::Relaxed);
    engine.quick_telex = config.quick_telex.load(Ordering::Relaxed);
    engine.restore_if_wrong_spelling = config.restore_if_wrong_spelling.load(Ordering::Relaxed);
    engine.fix_recommend_browser = config.fix_recommend_browser.load(Ordering::Relaxed);
    engine.use_macro = config.use_macro.load(Ordering::Relaxed);
    engine.use_macro_in_english = config.use_macro_in_english.load(Ordering::Relaxed);
    engine.auto_caps_macro = config.auto_caps_macro.load(Ordering::Relaxed);
    engine.upper_case_first_char = config.upper_case_first_char.load(Ordering::Relaxed);
    engine.allow_consonant_zfwj = config.allow_consonant_zfwj.load(Ordering::Relaxed);
    engine.quick_start_consonant = config.quick_start_consonant.load(Ordering::Relaxed);
    engine.quick_end_consonant = config.quick_end_consonant.load(Ordering::Relaxed);
    engine.temp_off_spelling = config.temp_off_spelling.load(Ordering::Relaxed);
    engine.temp_off_openkey = config.temp_off_openkey.load(Ordering::Relaxed);
}

// ---------------------------------------------------------------- hotkeys

/// Low byte of a hotkey when it is made of modifiers only (e.g. Ctrl + Shift)
const MODIFIERS_ONLY: u8 = 0xFE;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Hotkey {
    pub ctrl: bool,
    pub alt: bool,
    pub win: bool,
    pub shift: bool,
    /// A-Z / 0-9 (their virtual key equals the ASCII code); None = modifiers only
    pub key: Option<char>,
    pub beep: bool,
}

pub fn decode_hotkey(status: u32) -> Hotkey {
    let key = (status & 0xFF) as u8;
    Hotkey {
        ctrl: (status & 0x100) != 0,
        alt: (status & 0x200) != 0,
        win: (status & 0x400) != 0,
        shift: (status & 0x800) != 0,
        key: key.is_ascii_alphanumeric().then(|| key.to_ascii_uppercase() as char),
        beep: (status & 0x8000) != 0,
    }
}

pub fn encode_hotkey(h: &Hotkey) -> u32 {
    let key = h
        .key
        .filter(|c| c.is_ascii_alphanumeric())
        .map_or(MODIFIERS_ONLY, |c| c.to_ascii_uppercase() as u8);
    let mut status = 0;
    if h.ctrl { status |= 0x100; }
    if h.alt { status |= 0x200; }
    if h.win { status |= 0x400; }
    if h.shift { status |= 0x800; }
    if status == 0 && key == MODIFIERS_ONLY {
        // Neither a key nor a modifier: no hotkey at all
        return EMPTY_HOTKEY;
    }
    if h.beep { status |= 0x8000; }
    // The key is stored in the low byte (used by the hook) and the high byte (OpenKey's dialogs)
    status | key as u32 | ((key as u32) << 24)
}

// ---------------------------------------------------------------- input method

pub fn set_language(ctx: &AppContext, lang: u32) {
    ctx.config.language.store(lang, Ordering::SeqCst);
    if let Ok(mut eng) = ctx.engine.lock() {
        eng.language = lang;
        eng.start_new_session();
    }
    if ctx.config.use_smart_switch_key.load(Ordering::Relaxed) {
        remember_for_front_app(ctx);
    }
    crate::tray::refresh_tray(ctx.tray_hwnd);
    ctx.config.save_to_registry();
}

pub fn set_input_type(ctx: &AppContext, idx: u32) {
    ctx.config.input_type.store(idx, Ordering::SeqCst);
    if let Ok(mut eng) = ctx.engine.lock() {
        eng.input_type = InputType::from_u32(idx);
        eng.start_new_session();
    }
    ctx.config.save_to_registry();
}

pub fn set_code_table(ctx: &AppContext, idx: u32) {
    ctx.config.code_table.store(idx, Ordering::SeqCst);
    if let Ok(mut eng) = ctx.engine.lock() {
        eng.code_table = idx as usize;
    }
    if ctx.config.remember_code.load(Ordering::Relaxed) {
        remember_for_front_app(ctx);
    }
    ctx.config.save_to_registry();
}

fn remember_for_front_app(ctx: &AppContext) {
    let Some(app) = crate::smart_switch::get_frontmost_app_name() else { return };
    if let Ok(mut table) = ctx.smart_switch.lock() {
        table.update(
            &app,
            ctx.config.language.load(Ordering::Relaxed),
            ctx.config.code_table.load(Ordering::Relaxed) as usize,
        );
        crate::config::set_reg_binary("smartSwitchKey", &table.to_binary());
    }
}

pub fn set_switch_hotkey(ctx: &AppContext, hotkey: &Hotkey) {
    ctx.config.switch_key_status.store(encode_hotkey(hotkey), Ordering::SeqCst);
    ctx.config.save_to_registry();
}

// ---------------------------------------------------------------- on/off options

/// What the UI must do after an option changed
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterSetting {
    Continue,
    /// A new elevated instance was launched: this one must exit
    Quit,
}

pub fn get_setting(config: &AppConfig, name: &str) -> bool {
    let flag = match name {
        "modern_orthography" => &config.use_modern_orthography,
        "fix_browser" => &config.fix_recommend_browser,
        "check_spelling" => &config.check_spelling,
        "restore_spelling" => &config.restore_if_wrong_spelling,
        "allow_zfwj" => &config.allow_consonant_zfwj,
        "temp_off_spelling" => &config.temp_off_spelling,
        "smart_switch" => &config.use_smart_switch_key,
        "upper_first" => &config.upper_case_first_char,
        "remember_code" => &config.remember_code,
        "other_language" => &config.other_language,
        "temp_off_openkey" => &config.temp_off_openkey,
        "quick_telex" => &config.quick_telex,
        "use_macro" => &config.use_macro,
        "macro_in_english" => &config.use_macro_in_english,
        "macro_auto_caps" => &config.auto_caps_macro,
        "quick_start_consonant" => &config.quick_start_consonant,
        "quick_end_consonant" => &config.quick_end_consonant,
        "modern_icon" => &config.use_gray_icon,
        "show_on_startup" => &config.show_on_startup,
        "run_with_windows" => &config.run_with_windows,
        "run_as_admin" => &config.run_as_admin,
        "check_update" => &config.check_new_version,
        "desktop_shortcut" => &config.create_desktop_shortcut,
        "fix_chromium" => &config.fix_chromium_browser,
        "support_metro" => &config.support_metro_app,
        // Shown as "use the clipboard", stored as its opposite
        "use_clipboard" => return !config.send_key_step_by_step.load(Ordering::Relaxed),
        _ => return false,
    };
    flag.load(Ordering::Relaxed)
}

pub fn apply_setting(ctx: &AppContext, name: &str, val: bool) -> AfterSetting {
    let config = &ctx.config;
    let mut after = AfterSetting::Continue;
    {
        let mut eng = ctx.engine.lock();
        let eng = eng.as_deref_mut().ok();
        match name {
            "modern_orthography" => {
                config.use_modern_orthography.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.use_modern_orthography = val; }
            }
            "fix_browser" => {
                config.fix_recommend_browser.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.fix_recommend_browser = val; }
            }
            "check_spelling" => {
                config.check_spelling.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.set_check_spelling(val); }
            }
            "restore_spelling" => {
                config.restore_if_wrong_spelling.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.restore_if_wrong_spelling = val; }
            }
            "allow_zfwj" => {
                config.allow_consonant_zfwj.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.allow_consonant_zfwj = val; }
            }
            "temp_off_spelling" => {
                config.temp_off_spelling.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.temp_off_spelling = val; }
            }
            "upper_first" => {
                config.upper_case_first_char.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.upper_case_first_char = val; }
            }
            "temp_off_openkey" => {
                config.temp_off_openkey.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.temp_off_openkey = val; }
            }
            "quick_telex" => {
                config.quick_telex.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.quick_telex = val; }
            }
            "use_macro" => {
                config.use_macro.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.use_macro = val; }
            }
            "macro_in_english" => {
                config.use_macro_in_english.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.use_macro_in_english = val; }
            }
            "macro_auto_caps" => {
                config.auto_caps_macro.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.auto_caps_macro = val; }
            }
            "quick_start_consonant" => {
                config.quick_start_consonant.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.quick_start_consonant = val; }
            }
            "quick_end_consonant" => {
                config.quick_end_consonant.store(val, Ordering::Relaxed);
                if let Some(e) = eng { e.quick_end_consonant = val; }
            }
            "smart_switch" => config.use_smart_switch_key.store(val, Ordering::Relaxed),
            "remember_code" => config.remember_code.store(val, Ordering::Relaxed),
            "other_language" => config.other_language.store(val, Ordering::Relaxed),
            "show_on_startup" => config.show_on_startup.store(val, Ordering::Relaxed),
            "check_update" => config.check_new_version.store(val, Ordering::Relaxed),
            "fix_chromium" => config.fix_chromium_browser.store(val, Ordering::Relaxed),
            "support_metro" => config.support_metro_app.store(val, Ordering::Relaxed),
            "use_clipboard" => config.send_key_step_by_step.store(!val, Ordering::Relaxed),
            "modern_icon" => config.use_gray_icon.store(val, Ordering::Relaxed),
            "run_with_windows" => config.run_with_windows.store(val, Ordering::Relaxed),
            "run_as_admin" => config.run_as_admin.store(val, Ordering::Relaxed),
            "desktop_shortcut" => config.create_desktop_shortcut.store(val, Ordering::Relaxed),
            _ => {}
        }
    }

    // Side effects outside the engine (the engine lock is released: they may block or show dialogs)
    match name {
        "modern_icon" => crate::tray::refresh_tray(ctx.tray_hwnd),
        "run_with_windows" => register_run_on_startup(val, config.run_as_admin.load(Ordering::Relaxed)),
        "run_as_admin" => {
            if val && !crate::config::is_user_an_admin() {
                if crate::dialog::confirm(
                    "Minkey",
                    "Bạn cần phải khởi động lại Minkey để kích hoạt chế độ Admin!\nBạn có muốn khởi động lại Minkey không?",
                ) {
                    config.save_to_registry();
                    crate::config::relaunch_as_admin();
                    after = AfterSetting::Quit;
                }
            } else {
                register_run_on_startup(config.run_with_windows.load(Ordering::Relaxed), val);
            }
        }
        "desktop_shortcut" if val => crate::config::create_desktop_shortcut(),
        _ => {}
    }
    config.save_to_registry();
    after
}

/// Restores the factory settings of the input method (conversion tool settings are kept)
pub fn reset_defaults(ctx: &AppContext) {
    let config = &ctx.config;
    let def = AppConfig::default();
    macro_rules! reset {
        ($($field:ident),* $(,)?) => {
            $( config.$field.store(def.$field.load(Ordering::Relaxed), Ordering::Relaxed); )*
        };
    }
    reset!(
        language, input_type, code_table, check_spelling, use_modern_orthography, quick_telex,
        restore_if_wrong_spelling, fix_recommend_browser, use_macro, use_macro_in_english, auto_caps_macro,
        send_key_step_by_step, use_gray_icon, show_on_startup, run_with_windows, use_smart_switch_key,
        upper_case_first_char, allow_consonant_zfwj, temp_off_spelling, quick_start_consonant,
        quick_end_consonant, support_metro_app, run_as_admin, create_desktop_shortcut, check_new_version,
        remember_code, other_language, temp_off_openkey, fix_chromium_browser,
    );
    config.switch_key_status.store(DEFAULT_SWITCH_STATUS, Ordering::Relaxed);

    if let Ok(mut eng) = ctx.engine.lock() {
        sync_engine_from_config(&mut eng, config);
        eng.start_new_session();
    }
    crate::tray::refresh_tray(ctx.tray_hwnd);
    config.save_to_registry();
}

// ---------------------------------------------------------------- macros

fn with_macro_table<R>(ctx: &AppContext, f: impl FnOnce(&mut MacroTable) -> R) -> Option<R> {
    // Clone the table handle so the engine (and so the keyboard hook) is not blocked meanwhile
    let table = ctx.engine.lock().ok()?.macro_table.clone();
    let mut table = table.lock().ok()?;
    Some(f(&mut table))
}

fn save_macros(table: &MacroTable) {
    crate::config::set_reg_binary("macroData", &table.to_binary());
}

/// Macros whose shorthand or text contains `filter` (case-insensitive), sorted by shorthand
pub fn macro_entries(ctx: &AppContext, filter: &str) -> Vec<MacroEntry> {
    let filter = filter.trim().to_lowercase();
    with_macro_table(ctx, |t| t.get_all())
        .unwrap_or_default()
        .into_iter()
        .filter(|m| {
            filter.is_empty() || m.text.to_lowercase().contains(&filter) || m.content.to_lowercase().contains(&filter)
        })
        .collect()
}

pub fn macro_content(ctx: &AppContext, text: &str) -> Option<String> {
    with_macro_table(ctx, |t| t.map.get(text.trim()).cloned()).flatten()
}

/// Adds the macro or replaces its text; false when a field is empty
pub fn save_macro(ctx: &AppContext, text: &str, content: &str) -> bool {
    let (text, content) = (text.trim(), content.trim());
    if text.is_empty() || content.is_empty() {
        return false;
    }
    with_macro_table(ctx, |t| {
        t.add(text, content);
        save_macros(t);
    })
    .is_some()
}

pub fn delete_macro(ctx: &AppContext, text: &str) -> bool {
    with_macro_table(ctx, |t| {
        let deleted = t.delete(text);
        if deleted {
            save_macros(t);
        }
        deleted
    })
    .unwrap_or(false)
}

pub fn import_macros(ctx: &AppContext, path: &str, keep_existing: bool) -> std::io::Result<usize> {
    with_macro_table(ctx, |t| {
        let added = t.import_file(path, keep_existing)?;
        save_macros(t);
        Ok(added)
    })
    .unwrap_or(Ok(0))
}

pub fn export_macros(ctx: &AppContext, path: &str) -> std::io::Result<()> {
    with_macro_table(ctx, |t| t.export_file(path)).unwrap_or(Ok(()))
}

// ---------------------------------------------------------------- conversion tool

pub fn convert_options(config: &AppConfig) -> ConvertOptions {
    ConvertOptions {
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

pub fn set_convert_codes(config: &AppConfig, from: u32, to: u32) {
    config.convert_from_code.store(from, Ordering::Relaxed);
    config.convert_to_code.store(to, Ordering::Relaxed);
    config.save_to_registry();
}

/// Sets one conversion option. The four letter-case options exclude each other (as in OpenKey).
pub fn set_convert_option(config: &AppConfig, name: &str, val: bool) {
    let case_options = [
        ("to_all_caps", &config.convert_to_all_caps),
        ("to_all_non_caps", &config.convert_to_all_non_caps),
        ("to_caps_each_word", &config.convert_to_caps_each_word),
        ("to_caps_first_letter", &config.convert_to_caps_first_letter),
    ];
    if let Some((_, flag)) = case_options.iter().find(|(n, _)| *n == name) {
        flag.store(val, Ordering::Relaxed);
        if val {
            for (n, other) in &case_options {
                if *n != name {
                    other.store(false, Ordering::Relaxed);
                }
            }
        }
    } else {
        match name {
            "remove_mark" => config.convert_remove_mark.store(val, Ordering::Relaxed),
            "dont_alert" => config.convert_dont_alert.store(val, Ordering::Relaxed),
            _ => {}
        }
    }
    config.save_to_registry();
}

pub fn set_convert_hotkey(config: &AppConfig, hotkey: &Hotkey) {
    config.convert_hotkey.store(encode_hotkey(&Hotkey { beep: false, ..*hotkey }), Ordering::SeqCst);
    config.save_to_registry();
}

/// Converts the clipboard with the saved options, then confirms (message box or beep)
pub fn convert_clipboard(config: &AppConfig) {
    let opts = convert_options(config);
    if crate::convert::quick_convert_clipboard(&opts) {
        if opts.dont_alert {
            crate::hook::MessageBeep(0);
        } else {
            crate::dialog::alert("Minkey - Chuyển mã", "Đã chuyển mã nội dung Clipboard thành công!");
        }
    }
}
