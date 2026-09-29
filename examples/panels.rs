// Opens the three control panels without the tray icon or the keyboard hook, to check the UI
// while another input method (or Minkey itself) is running:  cargo run --example panels
//
// It reads the real settings; changing a control in the preview does save it.

#![windows_subsystem = "windows"]

use std::sync::{Arc, Mutex};

fn main() {
    let config = Arc::new(minkey::config::AppConfig::load_from_registry());
    let engine = Arc::new(Mutex::new(minkey::VietnameseEngine::new()));
    if let Some(bytes) = minkey::config::get_reg_binary("macroData") {
        let table = engine.lock().unwrap().macro_table.clone();
        *table.lock().unwrap() = minkey::macro_engine::MacroTable::from_binary(&bytes);
    }
    let ctx = minkey::app::AppContext {
        config,
        engine,
        smart_switch: Arc::new(Mutex::new(minkey::smart_switch::SmartSwitchTable::new())),
        tray_hwnd: 0,
    };
    minkey::win32_ui::run_with(ctx, None);
}
