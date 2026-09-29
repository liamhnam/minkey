// Windows Low-Level Keyboard and Mouse Hook Manager
// Ported directly from OpenKey.cpp

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32};
#[cfg(windows)]
use std::sync::atomic::Ordering;
#[cfg(windows)]
use std::sync::Mutex;
#[cfg(windows)]
use std::thread;

#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_HIGHEST};
#[cfg(windows)]
use windows_sys::Win32::UI::Input::Ime::*;
#[cfg(windows)]
use windows_sys::Win32::UI::Input::KeyboardAndMouse::*;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(windows)]
use crate::tables::*;
#[cfg(windows)]
use crate::types::{
    CAPS_MASK, CHAR_CODE_MASK, HookCodeState, KeyEvent, KeyEventState,
};
#[cfg(windows)]
use crate::VietnameseEngine;

#[cfg(windows)]
#[link(name = "user32")]
unsafe extern "system" {
    pub safe fn MessageBeep(u_type: u32) -> i32;
    pub fn SetWinEventHook(
        event_min: u32,
        event_max: u32,
        hmod_win_event_proc: HMODULE,
        pfn_win_event_proc: Option<unsafe extern "system" fn(isize, u32, HWND, i32, i32, u32, u32)>,
        id_process: u32,
        id_thread: u32,
        dw_flags: u32,
    ) -> isize;
    pub fn UnhookWinEvent(h_win_event_hook: isize) -> i32;
}

#[cfg(target_os = "macos")]
#[allow(non_snake_case)]
pub fn MessageBeep(_u_type: u32) -> i32 {
    #[link(name = "AppKit", kind = "framework")]
    unsafe extern "C" {
        fn NSBeep();
    }
    unsafe { NSBeep() };
    0
}

pub const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;
pub const WINEVENT_SKIPOWNPROCESS: u32 = 0x0002;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn GetCurrentThreadId() -> u32;
}



// Windows IME Message constant
pub const IMC_GETOPENSTATUS: u32 = 0x0005;

// Modifier flags
pub const MASK_SHIFT: u16 = 0x01;
pub const MASK_CONTROL: u16 = 0x02;
pub const MASK_ALT: u16 = 0x04;
pub const MASK_WIN: u16 = 0x08;
pub const MASK_NUMLOCK: u16 = 0x10;
pub const MASK_CAPITAL: u16 = 0x20;
pub const MASK_SCROLL: u16 = 0x40;

pub use crate::config::DEFAULT_SWITCH_STATUS;
pub const EMPTY_HOTKEY: u32 = 0x00FE;

#[derive(Debug, Clone)]
pub struct HookConfig {
    pub language: Arc<AtomicU32>,                  // 0: English, 1: Vietnamese
    pub switch_key_status: Arc<AtomicU32>,         // Hotkey bitmask
    pub fix_recommend_browser: Arc<AtomicBool>,    // Sửa gợi ý trình duyệt
    pub fix_chromium_browser: Arc<AtomicBool>,     // Fix Chromium
    pub support_metro_app: Arc<AtomicBool>,        // Metro apps
    pub send_key_step_by_step: Arc<AtomicBool>,    // SendInput step by step
    pub temp_off_spelling: Arc<AtomicBool>,        // Tạm tắt kiểm tra chính tả
    pub temp_off_openkey: Arc<AtomicBool>,         // Tạm tắt bộ gõ
    pub use_smart_switch_key: Arc<AtomicBool>,     // Smart switch
    pub remember_code: Arc<AtomicBool>,            // Ghi nhớ bảng mã theo ứng dụng
    pub convert_tool_hotkey: Arc<AtomicU32>,       // Convert tool hotkey
    pub use_macro: Arc<AtomicBool>,                // Bật gõ tắt
    pub use_macro_in_english: Arc<AtomicBool>,     // Bật gõ tắt ở tiếng Anh
}

impl Default for HookConfig {
    fn default() -> Self {
        Self {
            language: Arc::new(AtomicU32::new(1)),
            switch_key_status: Arc::new(AtomicU32::new(DEFAULT_SWITCH_STATUS)),
            fix_recommend_browser: Arc::new(AtomicBool::new(true)),
            fix_chromium_browser: Arc::new(AtomicBool::new(false)),
            support_metro_app: Arc::new(AtomicBool::new(false)),
            send_key_step_by_step: Arc::new(AtomicBool::new(true)),
            temp_off_spelling: Arc::new(AtomicBool::new(false)),
            temp_off_openkey: Arc::new(AtomicBool::new(false)),
            use_smart_switch_key: Arc::new(AtomicBool::new(true)),
            remember_code: Arc::new(AtomicBool::new(true)),
            convert_tool_hotkey: Arc::new(AtomicU32::new(EMPTY_HOTKEY)),
            use_macro: Arc::new(AtomicBool::new(true)),
            use_macro_in_english: Arc::new(AtomicBool::new(false)),
        }
    }
}

#[cfg(windows)]
pub type PlatformHookService = WindowsHookService;

#[cfg(not(windows))]
pub use crate::hook_macos::MacHookService;

#[cfg(not(windows))]
pub type PlatformHookService = MacHookService;

#[cfg(not(windows))]
pub type WindowsHookService = MacHookService;

// Global hook state singleton accessed from Windows callback procedures
#[cfg(windows)]
static mut GLOBAL_HOOK: *mut HookStateContext = std::ptr::null_mut();

#[cfg(windows)]
struct HookStateContext {

    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    flag: u16,
    last_flag: u16,
    keycode: u16,
    is_flag_key: bool,
    has_just_used_hotkey: bool,
    sync_key: Vec<u8>,
    keyboard_hook: HHOOK,
    mouse_hook: HHOOK,
    win_event_hook: isize,
    on_language_changed: Option<Box<dyn Fn(u32) + Send + Sync>>,
    on_code_table_changed: Option<Box<dyn Fn(u32) + Send + Sync>>,
    on_quick_convert: Option<Box<dyn Fn() + Send + Sync>>,
}

#[cfg(windows)]
pub struct WindowsHookService {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    thread_handle: Option<thread::JoinHandle<()>>,
    thread_id: Arc<AtomicU32>,
}

#[cfg(windows)]
impl WindowsHookService {
    pub fn new(

        config: HookConfig,
        engine: Arc<Mutex<VietnameseEngine>>,
        smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    ) -> Self {
        Self {
            config,
            engine,
            smart_switch,
            thread_handle: None,
            thread_id: Arc::new(AtomicU32::new(0)),
        }
    }

    pub fn start<F1, F2, F3>(&mut self, on_lang_changed: F1, on_code_table_changed: F2, on_quick_convert: F3)
    where
        F1: Fn(u32) + Send + Sync + 'static,
        F2: Fn(u32) + Send + Sync + 'static,
        F3: Fn() + Send + Sync + 'static,
    {
        let config = self.config.clone();
        let engine = self.engine.clone();
        let smart_switch = self.smart_switch.clone();
        let thread_id_atomic = self.thread_id.clone();

        let handle = thread::spawn(move || {
            // Set thread priority to highest for zero latency typing
            unsafe {
                SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_HIGHEST);
            }

            let tid = unsafe { GetCurrentThreadId() };
            thread_id_atomic.store(tid, Ordering::SeqCst);

            unsafe {
                let h_instance = GetModuleHandleW(std::ptr::null());
                let kb_hook = SetWindowsHookExW(
                    WH_KEYBOARD_LL,
                    Some(keyboard_hook_proc),
                    h_instance,
                    0,
                );
                let ms_hook = SetWindowsHookExW(
                    WH_MOUSE_LL,
                    Some(mouse_hook_proc),
                    h_instance,
                    0,
                );
                let ev_hook = SetWinEventHook(
                    EVENT_SYSTEM_FOREGROUND,
                    EVENT_SYSTEM_FOREGROUND,
                    std::ptr::null_mut(),
                    Some(win_event_proc_callback),
                    0,
                    0,
                    WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS,
                );

                let mut initial_flag: u16 = 0;
                if GetKeyState(VK_LSHIFT as i32) < 0 || GetKeyState(VK_RSHIFT as i32) < 0 { initial_flag |= MASK_SHIFT; }
                if GetKeyState(VK_LCONTROL as i32) < 0 || GetKeyState(VK_RCONTROL as i32) < 0 { initial_flag |= MASK_CONTROL; }
                if GetKeyState(VK_LMENU as i32) < 0 || GetKeyState(VK_RMENU as i32) < 0 { initial_flag |= MASK_ALT; }
                if GetKeyState(VK_LWIN as i32) < 0 || GetKeyState(VK_RWIN as i32) < 0 { initial_flag |= MASK_WIN; }
                if GetKeyState(VK_NUMLOCK as i32) < 0 { initial_flag |= MASK_NUMLOCK; }
                if GetKeyState(VK_CAPITAL as i32) == 1 { initial_flag |= MASK_CAPITAL; }
                if GetKeyState(VK_SCROLL as i32) < 0 { initial_flag |= MASK_SCROLL; }

                let ctx = Box::new(HookStateContext {
                    config,
                    engine,
                    smart_switch,
                    flag: initial_flag,
                    last_flag: 0,
                    keycode: 0,
                    is_flag_key: false,
                    has_just_used_hotkey: false,
                    sync_key: Vec::new(),
                    keyboard_hook: kb_hook,
                    mouse_hook: ms_hook,
                    win_event_hook: ev_hook,
                    on_language_changed: Some(Box::new(on_lang_changed)),
                    on_code_table_changed: Some(Box::new(on_code_table_changed)),
                    on_quick_convert: Some(Box::new(on_quick_convert)),
                });
                GLOBAL_HOOK = Box::into_raw(ctx);

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                if !GLOBAL_HOOK.is_null() {
                    let ctx = Box::from_raw(GLOBAL_HOOK);
                    GLOBAL_HOOK = std::ptr::null_mut();
                    UnhookWindowsHookEx(ctx.keyboard_hook);
                    UnhookWindowsHookEx(ctx.mouse_hook);
                    if ctx.win_event_hook != 0 {
                        UnhookWinEvent(ctx.win_event_hook);
                    }
                }
            }
        });

        self.thread_handle = Some(handle);
    }

    pub fn stop(&mut self) {
        let tid = self.thread_id.load(Ordering::SeqCst);
        if tid != 0 {
            unsafe {
                PostThreadMessageW(tid, WM_QUIT, 0, 0);
            }
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

// Low-level keyboard hook callback procedure
#[cfg(windows)]
unsafe extern "system" fn keyboard_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code < 0 {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    }

    if unsafe { GLOBAL_HOOK.is_null() } {
        return unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) };
    }
    let hook_ctx = unsafe { &mut *GLOBAL_HOOK };


    let kbd = unsafe { *(l_param as *const KBDLLHOOKSTRUCT) };

    // Ignore events injected by ourselves (dwExtraInfo != 0)
    if kbd.dwExtraInfo != 0 {
        return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
    }

    // Ignore if IME pad is open (Japanese/Chinese input)
    let hwnd_fore = unsafe { GetForegroundWindow() };
    let h_ime = unsafe { ImmGetDefaultIMEWnd(hwnd_fore) };
    if !h_ime.is_null() {
        let is_ime_on = unsafe { SendMessageW(h_ime, WM_IME_CONTROL as u32, IMC_GETOPENSTATUS as usize, 0) };
        if is_ime_on != 0 {
            return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
        }
    }

    let vk = kbd.vkCode as u16;
    let is_key_down = w_param == WM_KEYDOWN as usize || w_param == WM_SYSKEYDOWN as usize;
    let is_key_up = w_param == WM_KEYUP as usize || w_param == WM_SYSKEYUP as usize;

    sync_modifier_mask(hook_ctx, vk);
    if is_key_down {
        set_modifier_mask(hook_ctx, vk);
    } else if is_key_up {
        unset_modifier_mask(hook_ctx, vk);
    }

    if !hook_ctx.is_flag_key && !is_key_up {
        hook_ctx.keycode = vk;
    }

    // Check hotkeys for language switch / quick convert
    let switch_hotkey = hook_ctx.config.switch_key_status.load(Ordering::Relaxed);
    let convert_hotkey = hook_ctx.config.convert_tool_hotkey.load(Ordering::Relaxed);

    if is_key_down && !hook_ctx.is_flag_key && hook_ctx.keycode != 0 {
        let switch_vk = (switch_hotkey & 0xFF) as u16;
        let convert_vk = (convert_hotkey & 0xFF) as u16;

        if switch_vk != hook_ctx.keycode && convert_vk != hook_ctx.keycode {
            hook_ctx.last_flag = 0;
        } else {
            if switch_vk == hook_ctx.keycode && check_hot_key(switch_hotkey, hook_ctx.flag, hook_ctx.keycode, switch_vk != 0xFE) {
                switch_language_internal(hook_ctx);
                hook_ctx.has_just_used_hotkey = true;
                hook_ctx.keycode = 0;
                return 1; // Consume event
            }
            if convert_vk == hook_ctx.keycode && check_hot_key(convert_hotkey, hook_ctx.flag, hook_ctx.keycode, convert_vk != 0xFE) {
                if let Some(cb) = &hook_ctx.on_quick_convert {
                    cb();
                }
                hook_ctx.has_just_used_hotkey = true;
                hook_ctx.keycode = 0;
                return 1; // Consume event
            }
        }
        hook_ctx.has_just_used_hotkey = hook_ctx.last_flag != 0;
    } else if hook_ctx.is_flag_key {
        if hook_ctx.last_flag == 0 || hook_ctx.last_flag < hook_ctx.flag {
            hook_ctx.last_flag = hook_ctx.flag;
        } else if hook_ctx.last_flag > hook_ctx.flag {
            // Check modifier key release (e.g. Ctrl+Shift)
            if check_hot_key(switch_hotkey, hook_ctx.last_flag, 0, (switch_hotkey & 0xFF) != 0xFE) {
                switch_language_internal(hook_ctx);
                hook_ctx.has_just_used_hotkey = true;
            }
            if check_hot_key(convert_hotkey, hook_ctx.last_flag, 0, (convert_hotkey & 0xFF) != 0xFE) {
                if let Some(cb) = &hook_ctx.on_quick_convert {
                    cb();
                }
                hook_ctx.has_just_used_hotkey = true;
            }

            // Temp off spelling with Ctrl
            if hook_ctx.config.temp_off_spelling.load(Ordering::Relaxed) && !hook_ctx.has_just_used_hotkey && (hook_ctx.last_flag & MASK_CONTROL) != 0 {
                if let Ok(mut engine) = hook_ctx.engine.lock() {
                    engine.temp_off_spell_checking();
                }
            }
            // Temp off engine with Alt
            if hook_ctx.config.temp_off_openkey.load(Ordering::Relaxed) && !hook_ctx.has_just_used_hotkey && (hook_ctx.last_flag & MASK_ALT) != 0 {
                if let Ok(mut engine) = hook_ctx.engine.lock() {
                    let cur = engine.will_temp_off_engine;
                    engine.temp_off_engine(!cur);
                }
            }

            hook_ctx.last_flag = hook_ctx.flag;
            hook_ctx.has_just_used_hotkey = false;
        }
        hook_ctx.keycode = 0;
        return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
    }

    let language = hook_ctx.config.language.load(Ordering::Relaxed);
    // English mode
    if language == 0 {
        return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
    }

    // Vietnamese mode - only process on key down
    if is_key_down {
        let is_shift = (hook_ctx.flag & MASK_SHIFT) != 0;
        let is_caps_lock = (hook_ctx.flag & MASK_CAPITAL) != 0;
        let caps_status: u8 = if is_shift && is_caps_lock {
            0
        } else if is_shift {
            1
        } else if is_caps_lock {
            2
        } else {
            0
        };

        let other_ctrl = (hook_ctx.flag & (MASK_CONTROL | MASK_ALT | MASK_WIN)) != 0;

        let mut engine = match hook_ctx.engine.lock() {
            Ok(e) => e,
            Err(_) => return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) },
        };

        let state = engine.handle_event(
            KeyEvent::Keyboard,
            KeyEventState::KeyDown,
            hook_ctx.keycode,
            caps_status,
            other_ctrl,
        ).clone();

        match state.code {
            HookCodeState::DoNothing => {
                let code_table = engine.code_table;
                if is_double_code(code_table) {
                    if state.ext_code == 1 {
                        hook_ctx.sync_key.clear();
                    } else if state.ext_code == 2 {
                        if let Some(len) = hook_ctx.sync_key.pop() {
                            if len > 1 && (code_table == 2 || code_table == 3) {
                                unsafe { send_backspace() };
                            }
                        }
                    } else if state.ext_code == 3 {
                        hook_ctx.sync_key.push(1);
                    }
                }
                return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
            }
            HookCodeState::WillProcess | HookCodeState::Restore | HookCodeState::RestoreAndStartNewSession => {
                let code_table = engine.code_table;
                let mut bpc = state.backspace_count;

                // Browser autocomplete fix
                if hook_ctx.config.fix_recommend_browser.load(Ordering::Relaxed) && state.ext_code != 4 {
                    let mut use_chromium_fix = false;
                    if hook_ctx.config.fix_chromium_browser.load(Ordering::Relaxed) {
                        if let Some(last_app) = crate::smart_switch::get_last_app_name() {
                            if last_app.eq_ignore_ascii_case("chrome.exe")
                                || last_app.eq_ignore_ascii_case("brave.exe")
                                || last_app.eq_ignore_ascii_case("msedge.exe")
                            {
                                use_chromium_fix = true;
                            }
                        }
                    }

                    if use_chromium_fix {
                        unsafe { send_combine_key(VK_LSHIFT, VK_LEFT, 0, KEYEVENTF_EXTENDEDKEY); }
                        if bpc == 1 {
                            bpc -= 1;
                        }
                    } else {
                        unsafe { send_empty_character(code_table, &mut hook_ctx.sync_key) };
                        bpc += 1;
                    }
                }

                // Send backspaces
                for _ in 0..bpc {
                    unsafe { send_backspace() };
                    if is_double_code(code_table) && !hook_ctx.sync_key.is_empty() {
                        if hook_ctx.sync_key.pop().unwrap_or(1) > 1 {
                            unsafe { send_backspace() };
                        }
                    }
                }

                // Send new characters
                let ncc = state.new_char_count as usize;
                for i in (0..ncc).rev() {
                    unsafe { send_key_code(state.char_data[i], code_table, &mut hook_ctx.sync_key) };
                }

                if state.code == HookCodeState::Restore || state.code == HookCodeState::RestoreAndStartNewSession {
                    let k = hook_ctx.keycode as u32 | if caps_status > 0 { CAPS_MASK } else { 0 };
                    unsafe { send_key_code(k, code_table, &mut hook_ctx.sync_key) };
                }

                if state.code == HookCodeState::RestoreAndStartNewSession {
                    engine.start_new_session();
                }

                return 1; // Consume key event
            }
            HookCodeState::ReplaceMacro => {
                let code_table = engine.code_table;
                let mut bpc = state.backspace_count;
                if hook_ctx.config.fix_recommend_browser.load(Ordering::Relaxed) {
                    unsafe { send_empty_character(code_table, &mut hook_ctx.sync_key) };
                    bpc += 1;
                }
                for _ in 0..bpc {
                    unsafe { send_backspace() };
                    if is_double_code(code_table) && !hook_ctx.sync_key.is_empty() {
                        if hook_ctx.sync_key.pop().unwrap_or(1) > 1 {
                            unsafe { send_backspace() };
                        }
                    }
                }
                for &m_code in state.macro_data.iter() {
                    unsafe { send_key_code(m_code, code_table, &mut hook_ctx.sync_key) };
                }
                // The space / punctuation / Enter that triggered the macro was consumed: type it after the text
                let k = hook_ctx.keycode as u32 | if is_shift { CAPS_MASK } else { 0 };
                unsafe { send_key_code(k, code_table, &mut hook_ctx.sync_key) };
                return 1; // Consume key event
            }
            HookCodeState::BreakWord => {
                return unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) };
            }
        }
    }

    unsafe { CallNextHookEx(hook_ctx.keyboard_hook, n_code, w_param, l_param) }
}

// Low-level mouse hook callback procedure (breaks typing session on mouse click)
#[cfg(windows)]
unsafe extern "system" fn mouse_hook_proc(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code >= 0 {
        if w_param == WM_LBUTTONDOWN as usize || w_param == WM_RBUTTONDOWN as usize || w_param == WM_MBUTTONDOWN as usize {
            if !unsafe { GLOBAL_HOOK.is_null() } {
                let ctx = unsafe { &mut *GLOBAL_HOOK };
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.handle_event(KeyEvent::Mouse, KeyEventState::MouseDown, 0, 0, false);
                    if is_double_code(engine.code_table) {
                        ctx.sync_key.clear();
                    }
                }
            }
        }

    }
    unsafe { CallNextHookEx(std::ptr::null_mut(), n_code, w_param, l_param) }
}

// Foreground window event callback for Smart Switch Key and App Memory
#[cfg(windows)]
unsafe extern "system" fn win_event_proc_callback(
    _h_win_event_hook: isize,
    _dw_event: u32,
    _hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _dw_event_thread: u32,
    _dwms_event_time: u32,
) {
    if unsafe { GLOBAL_HOOK.is_null() } {
        return;
    }
    let hook_ctx = unsafe { &mut *GLOBAL_HOOK };

    let use_smart = hook_ctx.config.use_smart_switch_key.load(Ordering::Relaxed);
    let remember_code = hook_ctx.config.remember_code.load(Ordering::Relaxed);

    if use_smart || remember_code {
        let exe = match crate::smart_switch::get_frontmost_app_name() {
            Some(name) => name,
            None => return,
        };
        if exe.eq_ignore_ascii_case("explorer.exe") {
            return;
        }

        let cur_lang = hook_ctx.config.language.load(Ordering::Relaxed);
        let cur_code_table = if let Ok(eng) = hook_ctx.engine.lock() {
            eng.code_table
        } else {
            0
        };

        let current_input_method = ((cur_lang & 0x01) as u8) | (((cur_code_table.min(4) as u8) << 1) & 0xFE);

        let mut should_save = false;
        let target_val = if let Ok(mut table) = hook_ctx.smart_switch.lock() {
            let res = table.get_app_input_method_status(&exe, current_input_method);
            if res == -1 {
                should_save = true;
                current_input_method as i32
            } else {
                res
            }
        } else {
            -1
        };

        if should_save {
            if let Ok(table) = hook_ctx.smart_switch.lock() {
                let bin = table.to_binary();
                crate::config::set_reg_binary("smartSwitchKey", &bin);
            }
        }

        if let Ok(mut eng) = hook_ctx.engine.lock() {
            eng.temp_off_engine(false);
            eng.start_new_session();
        }

        if target_val >= 0 {
            let target_lang = (target_val as u32) & 0x01;
            let target_table = ((target_val as usize) >> 1) & 0x07;

            if use_smart && target_lang != cur_lang {
                hook_ctx.config.language.store(target_lang, Ordering::SeqCst);
                if let Ok(mut eng) = hook_ctx.engine.lock() {
                    eng.language = target_lang;
                }
                if let Some(cb) = &hook_ctx.on_language_changed {
                    cb(target_lang);
                }
            }

            if remember_code && target_table != cur_code_table {
                if let Ok(mut eng) = hook_ctx.engine.lock() {
                    eng.code_table = target_table;
                }
                if let Some(cb) = &hook_ctx.on_code_table_changed {
                    cb(target_table as u32);
                }
            }
        }

        // Metro App support (ApplicationFrameHost.exe)
        if hook_ctx.config.support_metro_app.load(Ordering::Relaxed)
            && exe.eq_ignore_ascii_case("ApplicationFrameHost.exe")
        {
            unsafe {
                PostMessageW(0xFFFF as HWND, WM_CHAR, VK_BACK as usize, 0);
                PostMessageW(0xFFFF as HWND, WM_CHAR, VK_BACK as usize, 0);
            }
        }
    }
}

#[cfg(windows)]
fn set_modifier_mask(ctx: &mut HookStateContext, vk: u16) {
    unsafe {
        if GetKeyState(VK_CAPITAL as i32) == 1 { ctx.flag |= MASK_CAPITAL; } else { ctx.flag &= !MASK_CAPITAL; }
    }
    match vk {
        VK_LSHIFT | VK_RSHIFT => { ctx.flag |= MASK_SHIFT; ctx.is_flag_key = true; }
        VK_LCONTROL | VK_RCONTROL => { ctx.flag |= MASK_CONTROL; ctx.is_flag_key = true; }
        VK_LMENU | VK_RMENU => { ctx.flag |= MASK_ALT; ctx.is_flag_key = true; }
        VK_LWIN | VK_RWIN => { ctx.flag |= MASK_WIN; ctx.is_flag_key = true; }
        VK_NUMLOCK => { ctx.flag |= MASK_NUMLOCK; ctx.is_flag_key = true; }
        VK_SCROLL => { ctx.flag |= MASK_SCROLL; ctx.is_flag_key = true; }
        _ => { ctx.is_flag_key = false; }
    }
}

/// Re-reads Shift / Ctrl / Alt / Win from the keyboard state instead of trusting the key-ups seen so
/// far: a key-up can be swallowed (Ctrl+Alt+Del hands it to the secure desktop), which left the
/// modifier "down" forever — no Vietnamese while Ctrl/Alt/Win looked held, upper-case output while
/// Shift did. The key of the event being processed is skipped: a low-level hook runs before the
/// system updates that key's state; set/unset_modifier_mask handle it.
#[cfg(windows)]
fn sync_modifier_mask(ctx: &mut HookStateContext, vk: u16) {
    const MODIFIERS: [(u16, u16, u16); 4] = [
        (VK_LSHIFT, VK_RSHIFT, MASK_SHIFT),
        (VK_LCONTROL, VK_RCONTROL, MASK_CONTROL),
        (VK_LMENU, VK_RMENU, MASK_ALT),
        (VK_LWIN, VK_RWIN, MASK_WIN),
    ];
    let is_down = |key: u16| unsafe { GetAsyncKeyState(key as i32) } < 0;
    for (left, right, mask) in MODIFIERS {
        if vk == left || vk == right {
            continue;
        }
        if is_down(left) || is_down(right) {
            ctx.flag |= mask;
        } else {
            ctx.flag &= !mask;
        }
    }
}

#[cfg(windows)]
fn unset_modifier_mask(ctx: &mut HookStateContext, vk: u16) {
    match vk {
        VK_LSHIFT | VK_RSHIFT => { ctx.flag &= !MASK_SHIFT; ctx.is_flag_key = true; }
        VK_LCONTROL | VK_RCONTROL => { ctx.flag &= !MASK_CONTROL; ctx.is_flag_key = true; }
        VK_LMENU | VK_RMENU => { ctx.flag &= !MASK_ALT; ctx.is_flag_key = true; }
        VK_LWIN | VK_RWIN => { ctx.flag &= !MASK_WIN; ctx.is_flag_key = true; }
        VK_NUMLOCK => { ctx.flag &= !MASK_NUMLOCK; ctx.is_flag_key = true; }
        VK_SCROLL => { ctx.flag &= !MASK_SCROLL; ctx.is_flag_key = true; }
        _ => { ctx.is_flag_key = false; }
    }
}

#[cfg(windows)]
fn check_hot_key(hotkey: u32, flag: u16, keycode: u16, check_code: bool) -> bool {
    if (hotkey & !0x8000) == EMPTY_HOTKEY {
        return false;
    }
    let has_ctrl = (hotkey & 0x100) != 0;
    let has_alt = (hotkey & 0x200) != 0;
    let has_win = (hotkey & 0x400) != 0;
    let has_shift = (hotkey & 0x800) != 0;

    let flag_ctrl = (flag & MASK_CONTROL) != 0;
    let flag_alt = (flag & MASK_ALT) != 0;
    let flag_win = (flag & MASK_WIN) != 0;
    let flag_shift = (flag & MASK_SHIFT) != 0;

    if has_ctrl != flag_ctrl || has_alt != flag_alt || has_win != flag_win || has_shift != flag_shift {
        return false;
    }

    if check_code {
        let code = (hotkey & 0xFF) as u16;
        if code != keycode {
            return false;
        }
    }
    true
}

#[cfg(windows)]
fn switch_language_internal(ctx: &mut HookStateContext) {
    let cur = ctx.config.language.load(Ordering::SeqCst);
    let next = if cur == 0 { 1 } else { 0 };
    ctx.config.language.store(next, Ordering::SeqCst);

    let hotkey = ctx.config.switch_key_status.load(Ordering::Relaxed);
    if (hotkey & 0x8000) != 0 {
        MessageBeep(MB_OK);
    }

    if let Ok(mut engine) = ctx.engine.lock() {
        engine.language = next;
        engine.start_new_session();
    }

    if ctx.config.use_smart_switch_key.load(Ordering::Relaxed) {
        if let Some(exe) = crate::smart_switch::get_frontmost_app_name() {
            let code_table = if let Ok(eng) = ctx.engine.lock() { eng.code_table } else { 0 };
            if let Ok(mut table) = ctx.smart_switch.lock() {
                table.update(&exe, next, code_table);
                let bin = table.to_binary();
                crate::config::set_reg_binary("smartSwitchKey", &bin);
            }
        }
    }

    if let Some(cb) = &ctx.on_language_changed {
        cb(next);
    }
}

#[cfg(windows)]
#[inline]
fn is_double_code(code_table: usize) -> bool {
    code_table == 2 || code_table == 3
}

// Low-level SendInput helpers
#[cfg(windows)]
unsafe fn send_backspace() {
    unsafe {
        let mut inputs = [std::mem::zeroed::<INPUT>(); 2];
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wVk = VK_BACK;
        inputs[0].Anonymous.ki.dwExtraInfo = 1;

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wVk = VK_BACK;
        inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
        inputs[1].Anonymous.ki.dwExtraInfo = 1;

        SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
unsafe fn send_empty_character(code_table: usize, sync_key: &mut Vec<u8>) {
    if is_double_code(code_table) {
        sync_key.push(1);
    }
    unsafe {
        let mut inputs = [std::mem::zeroed::<INPUT>(); 2];
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wScan = 0x202F;
        inputs[0].Anonymous.ki.dwFlags = KEYEVENTF_UNICODE;
        inputs[0].Anonymous.ki.dwExtraInfo = 1;

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wScan = 0x202F;
        inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
        inputs[1].Anonymous.ki.dwExtraInfo = 1;

        SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
unsafe fn send_unicode_char(ch: u16) {
    unsafe {
        let mut inputs = [std::mem::zeroed::<INPUT>(); 2];
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wScan = ch;
        inputs[0].Anonymous.ki.dwFlags = KEYEVENTF_UNICODE;
        inputs[0].Anonymous.ki.dwExtraInfo = 1;

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wScan = ch;
        inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_UNICODE | KEYEVENTF_KEYUP;
        inputs[1].Anonymous.ki.dwExtraInfo = 1;

        SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

#[cfg(windows)]
unsafe fn send_key_code(data: u32, code_table: usize, sync_key: &mut Vec<u8>) {
    // Macro text outside the key map / Vietnamese tables (™, →, “ ”, emoji...): it is a code point,
    // not a virtual key, so sending it as wVk would press an unrelated key
    if let Some(ch) = pure_character(data) {
        if is_double_code(code_table) {
            sync_key.push(1);
        }
        let mut buf = [0u16; 2];
        for &unit in ch.encode_utf16(&mut buf).iter() {
            unsafe { send_unicode_char(unit) };
        }
        return;
    }
    if (data & CHAR_CODE_MASK) == 0 {
        if is_double_code(code_table) {
            sync_key.push(1);
        }
        let ch = key_code_to_character(data);
        if ch == 0 {
            unsafe {
                let mut inputs = [std::mem::zeroed::<INPUT>(); 2];
                inputs[0].r#type = INPUT_KEYBOARD;
                inputs[0].Anonymous.ki.wVk = data as u16;
                inputs[0].Anonymous.ki.dwExtraInfo = 1;

                inputs[1].r#type = INPUT_KEYBOARD;
                inputs[1].Anonymous.ki.wVk = data as u16;
                inputs[1].Anonymous.ki.dwFlags = KEYEVENTF_KEYUP;
                inputs[1].Anonymous.ki.dwExtraInfo = 1;

                SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
            }
        } else {
            unsafe { send_unicode_char(ch) };
        }
    } else {
        let raw = (data & 0xFFFF) as u16;
        if code_table == 0 {
            // Unicode precomposed
            unsafe { send_unicode_char(raw) };
        } else if code_table == 1 || code_table == 2 || code_table == 4 {
            // TCVN3, VNI Windows, CP1258
            let hi = (raw >> 8) as u8;
            let lo = (raw & 0xFF) as u8;
            unsafe { send_unicode_char(lo as u16) };
            if hi > 32 {
                if code_table == 2 {
                    sync_key.push(2);
                }
                unsafe { send_unicode_char(hi as u16) };
            } else if code_table == 2 {
                sync_key.push(1);
            }
        } else if code_table == 3 {
            // Unicode Compound
            let hi = (raw >> 13) as usize;
            let base = raw & 0x1FFF;
            let mark = if hi > 0 && hi <= 5 { UNICODE_COMPOUND_MARK[hi - 1] } else { 0 };
            sync_key.push(if mark > 0 { 2 } else { 1 });
            unsafe {
                send_unicode_char(base);
                if mark > 0 {
                    send_unicode_char(mark);
                }
            }
        }
    }
}

#[cfg(windows)]
unsafe fn send_combine_key(key1: u16, key2: u16, flag1: u32, flag2: u32) {
    unsafe {
        let mut inputs = [std::mem::zeroed::<INPUT>(); 4];
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki.wVk = key1;
        inputs[0].Anonymous.ki.dwFlags = flag1;
        inputs[0].Anonymous.ki.dwExtraInfo = 1;

        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki.wVk = key2;
        inputs[1].Anonymous.ki.dwFlags = flag2;
        inputs[1].Anonymous.ki.dwExtraInfo = 1;

        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki.wVk = key2;
        inputs[2].Anonymous.ki.dwFlags = flag2 | KEYEVENTF_KEYUP;
        inputs[2].Anonymous.ki.dwExtraInfo = 1;

        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki.wVk = key1;
        inputs[3].Anonymous.ki.dwFlags = flag1 | KEYEVENTF_KEYUP;
        inputs[3].Anonymous.ki.dwExtraInfo = 1;

        SendInput(4, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
    }
}

