// Startup wiring: loads the configuration, builds the engine and starts the tray and keyboard hook.
// The control panel talks to the running services only through `AppContext` and `UiCommand`.

use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crate::config::AppConfig;
use crate::engine::VietnameseEngine;
use crate::hook::PlatformHookService;
use crate::smart_switch::SmartSwitchTable;
use crate::tray::{PlatformTrayService, TrayCallbacks};

/// Everything a settings screen needs to read or change the running input method
#[derive(Clone)]
pub struct AppContext {
    pub config: Arc<AppConfig>,
    pub engine: Arc<Mutex<VietnameseEngine>>,
    pub smart_switch: Arc<Mutex<SmartSwitchTable>>,
    pub tray_hwnd: usize,
}

/// Requests from the tray and the keyboard hook (other threads) to the UI thread
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiCommand {
    ShowControlPanel,
    ShowAbout,
    ShowMacroTable,
    ShowConvertTool,
    LanguageChanged(u32),
    CodeTableChanged(u32),
    Exit,
}

pub struct Services {
    hook: PlatformHookService,
    tray: PlatformTrayService,
}

impl Services {
    pub fn stop(mut self) {
        self.hook.stop();
        self.tray.stop();
    }
}

/// Loads the saved state and starts the tray icon and the keyboard hook.
/// `dispatch` is called from the tray / hook threads and must hand the command to the UI thread.
pub fn start(dispatch: impl Fn(UiCommand) + Send + Sync + 'static) -> (AppContext, Services) {
    let config = Arc::new(AppConfig::load_from_registry());

    let engine = Arc::new(Mutex::new(VietnameseEngine::new()));
    if let Ok(mut eng) = engine.lock() {
        crate::settings::sync_engine_from_config(&mut eng, &config);
        if let Some(bytes) = crate::config::get_reg_binary("macroData") {
            if let Ok(mut table) = eng.macro_table.lock() {
                *table = crate::macro_engine::MacroTable::from_binary(&bytes);
            }
        }
    }

    let smart_switch = Arc::new(Mutex::new(match crate::config::get_reg_binary("smartSwitchKey") {
        Some(bytes) => SmartSwitchTable::from_binary(&bytes),
        None => SmartSwitchTable::new(),
    }));

    let dispatch: Arc<dyn Fn(UiCommand) + Send + Sync> = Arc::new(dispatch);
    let hook_config = config.to_hook_config();

    let d = dispatch.clone();
    let callbacks = TrayCallbacks {
        on_open_control_panel: Box::new({ let d = d.clone(); move || d(UiCommand::ShowControlPanel) }),
        on_open_macro_table: Box::new({ let d = d.clone(); move || d(UiCommand::ShowMacroTable) }),
        on_open_convert_tool: Box::new({ let d = d.clone(); move || d(UiCommand::ShowConvertTool) }),
        on_open_about: Box::new({ let d = d.clone(); move || d(UiCommand::ShowAbout) }),
        on_exit: Box::new(move || d(UiCommand::Exit)),
    };
    let mut tray = PlatformTrayService::new(hook_config.clone(), engine.clone(), callbacks);
    tray.start(config.use_gray_icon.clone());
    let tray_hwnd = tray.get_hwnd();

    let mut hook = PlatformHookService::new(hook_config, engine.clone(), smart_switch.clone());
    let on_lang = dispatch.clone();
    let on_table = dispatch;
    let table_config = config.clone();
    let convert_config = config.clone();
    hook.start(
        move |lang| {
            crate::tray::refresh_tray(tray_hwnd);
            on_lang(UiCommand::LanguageChanged(lang));
        },
        move |code_table| {
            table_config.code_table.store(code_table, Ordering::Relaxed);
            crate::tray::refresh_tray(tray_hwnd);
            on_table(UiCommand::CodeTableChanged(code_table));
        },
        move || {
            // Off the hook thread so typing never waits on the clipboard or a message box
            let config = convert_config.clone();
            std::thread::spawn(move || crate::settings::convert_clipboard(&config));
        },
    );

    let ctx = AppContext { config, engine, smart_switch, tray_hwnd };
    (ctx, Services { hook, tray })
}
