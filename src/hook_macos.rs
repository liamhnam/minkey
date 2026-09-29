// macOS CoreGraphics Event Tap Keyboard Hook
// Re-implements OpenKey.mm (OpenKeyCallback) on top of the shared Vietnamese engine.
// The engine works with Windows virtual key codes, so macOS keycodes are translated
// with `mac_keycode_to_engine_key` before being handed to it.

#![allow(non_upper_case_globals)]

use std::ffi::c_void;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::hook::{EMPTY_HOTKEY, HookConfig};
use crate::tables::{UNICODE_COMPOUND_MARK, key_code_to_character};
use crate::types::*;
use crate::VietnameseEngine;

// CoreGraphics event types
const kCGEventLeftMouseDown: u32 = 1;
const kCGEventRightMouseDown: u32 = 3;
const kCGEventLeftMouseDragged: u32 = 6;
const kCGEventRightMouseDragged: u32 = 7;
const kCGEventKeyDown: u32 = 10;
const kCGEventKeyUp: u32 = 11;
const kCGEventFlagsChanged: u32 = 12;
const kCGEventOtherMouseDown: u32 = 25;
const kCGEventTapDisabledByTimeout: u32 = 0xFFFF_FFFE;
const kCGEventTapDisabledByUserInput: u32 = 0xFFFF_FFFF;

const kCGSessionEventTap: u32 = 1;
const kCGHeadInsertEventTap: u32 = 0;
const kCGEventTapOptionDefault: u32 = 0;
const kCGEventSourceStatePrivate: i32 = -1;

// CGEventField
const kCGKeyboardEventKeycode: u32 = 9;
const kCGEventSourceUserData: u32 = 42;

// CGEventFlags
const kCGEventFlagMaskAlphaShift: u64 = 0x0001_0000;
const kCGEventFlagMaskShift: u64 = 0x0002_0000;
const kCGEventFlagMaskControl: u64 = 0x0004_0000;
const kCGEventFlagMaskAlternate: u64 = 0x0008_0000;
const kCGEventFlagMaskCommand: u64 = 0x0010_0000;
const kCGEventFlagMaskNumericPad: u64 = 0x0020_0000;
const kCGEventFlagMaskHelp: u64 = 0x0040_0000;
const kCGEventFlagMaskSecondaryFn: u64 = 0x0080_0000;

const OTHER_CONTROL_MASK: u64 = kCGEventFlagMaskCommand
    | kCGEventFlagMaskControl
    | kCGEventFlagMaskAlternate
    | kCGEventFlagMaskSecondaryFn
    | kCGEventFlagMaskNumericPad
    | kCGEventFlagMaskHelp;

// macOS hardware keycodes used for synthetic events
const MAC_KEY_DELETE: u16 = 51;
const MAC_KEY_LEFT: u16 = 123;

/// Tag written into every event Minkey posts, so the tap can skip its own output
const MINKEY_EVENT_MARKER: i64 = 0x4D49_4E4B; // 'MINK'
/// CGEventKeyboardSetUnicodeString silently truncates long strings, send in chunks
const MAX_UNICODE_CHUNK: usize = 16;

type CGEventTapProxy = *mut c_void;
type CGEventRef = *mut c_void;
type CGEventSourceRef = *mut c_void;
type CFMachPortRef = *mut c_void;
type CFRunLoopRef = *mut c_void;
type CFRunLoopSourceRef = *mut c_void;
type CFTypeRef = *const c_void;

type CGEventTapCallBack =
    unsafe extern "C" fn(CGEventTapProxy, u32, CGEventRef, *mut c_void) -> CGEventRef;

#[link(name = "CoreGraphics", kind = "framework")]
#[link(name = "CoreFoundation", kind = "framework")]
#[link(name = "ApplicationServices", kind = "framework")]
unsafe extern "C" {
    fn CGEventTapCreate(
        tap: u32,
        place: u32,
        options: u32,
        events_of_interest: u64,
        callback: CGEventTapCallBack,
        refcon: *mut c_void,
    ) -> CFMachPortRef;
    fn CGEventTapEnable(tap: CFMachPortRef, enable: bool);
    fn CGEventTapPostEvent(proxy: CGEventTapProxy, event: CGEventRef);

    fn CGEventSourceCreate(state_id: i32) -> CGEventSourceRef;
    fn CGEventCreateKeyboardEvent(source: CGEventSourceRef, keycode: u16, keydown: bool) -> CGEventRef;
    fn CGEventCreateCopy(event: CGEventRef) -> CGEventRef;
    fn CGEventGetFlags(event: CGEventRef) -> u64;
    fn CGEventSetFlags(event: CGEventRef, flags: u64);
    fn CGEventGetIntegerValueField(event: CGEventRef, field: u32) -> i64;
    fn CGEventSetIntegerValueField(event: CGEventRef, field: u32, value: i64);
    fn CGEventKeyboardSetUnicodeString(event: CGEventRef, length: usize, string: *const u16);

    fn CFMachPortCreateRunLoopSource(allocator: *const c_void, port: CFMachPortRef, order: isize) -> CFRunLoopSourceRef;
    fn CFRunLoopGetCurrent() -> CFRunLoopRef;
    fn CFRunLoopAddSource(rl: CFRunLoopRef, source: CFRunLoopSourceRef, mode: CFTypeRef);
    fn CFRunLoopRun();
    fn CFRunLoopStop(rl: CFRunLoopRef);
    fn CFRelease(cf: CFTypeRef);
    fn CFDictionaryCreate(
        allocator: *const c_void,
        keys: *const CFTypeRef,
        values: *const CFTypeRef,
        num_values: isize,
        key_callbacks: *const c_void,
        value_callbacks: *const c_void,
    ) -> CFTypeRef;

    static kCFRunLoopCommonModes: CFTypeRef;
    static kCFBooleanTrue: CFTypeRef;
    static kCFTypeDictionaryKeyCallBacks: c_void;
    static kCFTypeDictionaryValueCallBacks: c_void;
    static kAXTrustedCheckOptionPrompt: CFTypeRef;

    fn AXIsProcessTrusted() -> bool;
    fn AXIsProcessTrustedWithOptions(options: CFTypeRef) -> bool;
}

/// Checks Accessibility permission; when missing, asks macOS to show its permission prompt.
pub fn ensure_accessibility_permission() -> bool {
    unsafe {
        if AXIsProcessTrusted() {
            return true;
        }
        let keys = [kAXTrustedCheckOptionPrompt];
        let values = [kCFBooleanTrue];
        let options = CFDictionaryCreate(
            std::ptr::null(),
            keys.as_ptr(),
            values.as_ptr(),
            1,
            &raw const kCFTypeDictionaryKeyCallBacks,
            &raw const kCFTypeDictionaryValueCallBacks,
        );
        let trusted = AXIsProcessTrustedWithOptions(options);
        if !options.is_null() {
            CFRelease(options);
        }
        trusted
    }
}

struct MacHookContext {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    on_language_changed: Box<dyn Fn(u32) + Send + Sync>,
    on_code_table_changed: Box<dyn Fn(u32) + Send + Sync>,
    on_quick_convert: Box<dyn Fn() + Send + Sync>,

    event_source: CGEventSourceRef,
    tap: CFMachPortRef,
    proxy: CGEventTapProxy,
    flag: u64,
    last_flag: u64,
    keycode: u16, // engine (Windows VK) key code of the current key
    has_just_used_hotkey: bool,
    sync_key: Vec<u8>,
    front_app: String,
}

static mut GLOBAL_MAC_HOOK: *mut MacHookContext = std::ptr::null_mut();
static RUN_LOOP: Mutex<usize> = Mutex::new(0);

pub struct MacHookService {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    thread_handle: Option<thread::JoinHandle<()>>,
}

impl MacHookService {
    pub fn new(
        config: HookConfig,
        engine: Arc<Mutex<VietnameseEngine>>,
        smart_switch: Arc<Mutex<crate::smart_switch::SmartSwitchTable>>,
    ) -> Self {
        Self { config, engine, smart_switch, thread_handle: None }
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

        let handle = thread::Builder::new()
            .name("minkey-macos-hook".to_string())
            .spawn(move || unsafe {
                // Event taps need Accessibility permission. Prompt once, then wait until granted
                // so the user does not have to restart Minkey after allowing it.
                if !ensure_accessibility_permission() {
                    eprintln!("[Minkey] Waiting for Accessibility permission (System Settings → Privacy & Security → Accessibility)...");
                    while !AXIsProcessTrusted() {
                        thread::sleep(std::time::Duration::from_secs(1));
                    }
                }

                let context = Box::new(MacHookContext {
                    config,
                    engine,
                    smart_switch,
                    on_language_changed: Box::new(on_lang_changed),
                    on_code_table_changed: Box::new(on_code_table_changed),
                    on_quick_convert: Box::new(on_quick_convert),
                    event_source: CGEventSourceCreate(kCGEventSourceStatePrivate),
                    tap: std::ptr::null_mut(),
                    proxy: std::ptr::null_mut(),
                    flag: 0,
                    last_flag: 0,
                    keycode: 0,
                    has_just_used_hotkey: false,
                    sync_key: Vec::new(),
                    front_app: String::new(),
                });
                GLOBAL_MAC_HOOK = Box::into_raw(context);

                let event_mask: u64 = (1 << kCGEventKeyDown)
                    | (1 << kCGEventKeyUp)
                    | (1 << kCGEventFlagsChanged)
                    | (1 << kCGEventLeftMouseDown)
                    | (1 << kCGEventRightMouseDown)
                    | (1 << kCGEventOtherMouseDown)
                    | (1 << kCGEventLeftMouseDragged)
                    | (1 << kCGEventRightMouseDragged);

                let tap = CGEventTapCreate(
                    kCGSessionEventTap,
                    kCGHeadInsertEventTap,
                    kCGEventTapOptionDefault,
                    event_mask,
                    mac_event_tap_callback,
                    std::ptr::null_mut(),
                );
                if tap.is_null() {
                    eprintln!("[Minkey] Cannot create CGEventTap. Please grant Accessibility permission in System Settings → Privacy & Security → Accessibility.");
                    return;
                }
                (*GLOBAL_MAC_HOOK).tap = tap;

                let source = CFMachPortCreateRunLoopSource(std::ptr::null(), tap, 0);
                let run_loop = CFRunLoopGetCurrent();
                if let Ok(mut rl) = RUN_LOOP.lock() {
                    *rl = run_loop as usize;
                }
                CFRunLoopAddSource(run_loop, source, kCFRunLoopCommonModes);
                CGEventTapEnable(tap, true);

                CFRunLoopRun();

                CGEventTapEnable(tap, false);
                CFRelease(source);
                CFRelease(tap);
            })
            .ok();

        self.thread_handle = handle;
    }

    pub fn stop(&mut self) {
        if let Ok(mut rl) = RUN_LOOP.lock() {
            if *rl != 0 {
                unsafe { CFRunLoopStop(*rl as CFRunLoopRef) };
                *rl = 0;
            }
        }
        // The hook thread may still be waiting for Accessibility permission; don't block exit on it.
        if let Some(handle) = self.thread_handle.take() {
            if handle.is_finished() {
                let _ = handle.join();
            }
        }
    }
}

#[inline]
fn is_double_code(code_table: usize) -> bool {
    code_table == 2 || code_table == 3
}

/// Apps which render Unicode compound marks as one glyph (one backspace deletes both code units)
fn is_unicode_compound_app(app: &str) -> bool {
    const APPS: [&str; 7] = [
        "com.apple.",
        "com.google.Chrome",
        "com.brave.Browser",
        "com.microsoft.edgemac.Dev",
        "com.microsoft.edgemac.Beta",
        "com.microsoft.Edge.Dev",
        "com.microsoft.Edge",
    ];
    APPS.iter().any(|p| app.starts_with(p))
}

fn is_chromium_app(app: &str) -> bool {
    app != "com.apple.Safari" && is_unicode_compound_app(app) && !app.starts_with("com.apple.")
}

const SPOTLIGHT: &str = "com.apple.Spotlight";

impl MacHookContext {
    unsafe fn post(&self, event: CGEventRef) {
        unsafe {
            CGEventSetIntegerValueField(event, kCGEventSourceUserData, MINKEY_EVENT_MARKER);
            CGEventTapPostEvent(self.proxy, event);
            CFRelease(event);
        }
    }

    unsafe fn post_key(&self, mac_keycode: u16, extra_flags: u64) {
        unsafe {
            for down in [true, false] {
                let ev = CGEventCreateKeyboardEvent(self.event_source, mac_keycode, down);
                if extra_flags != 0 {
                    CGEventSetFlags(ev, CGEventGetFlags(ev) | extra_flags);
                }
                self.post(ev);
            }
        }
    }

    unsafe fn post_unicode(&self, chars: &[u16]) {
        for chunk in chars.chunks(MAX_UNICODE_CHUNK) {
            unsafe {
                for down in [true, false] {
                    let ev = CGEventCreateKeyboardEvent(self.event_source, 0, down);
                    CGEventKeyboardSetUnicodeString(ev, chunk.len(), chunk.as_ptr());
                    self.post(ev);
                }
            }
        }
    }

    /// Re-sends a copy of the key the user pressed (after Minkey's own output)
    unsafe fn repost_original(&self, event: CGEventRef) {
        unsafe {
            let copy = CGEventCreateCopy(event);
            if !copy.is_null() {
                self.post(copy);
            }
        }
    }

    unsafe fn send_backspace(&mut self, code_table: usize) {
        unsafe {
            self.post_key(MAC_KEY_DELETE, 0);
            if is_double_code(code_table) {
                if self.sync_key.last().copied().unwrap_or(1) > 1
                    && !(code_table == 3 && is_unicode_compound_app(&self.front_app))
                {
                    self.post_key(MAC_KEY_DELETE, 0);
                }
                self.sync_key.pop();
            }
        }
    }

    unsafe fn send_shift_left(&mut self, code_table: usize) {
        unsafe {
            self.post_key(MAC_KEY_LEFT, kCGEventFlagMaskShift);
            if is_double_code(code_table) {
                if self.sync_key.last().copied().unwrap_or(1) > 1
                    && !(code_table == 3 && is_unicode_compound_app(&self.front_app))
                {
                    self.post_key(MAC_KEY_LEFT, kCGEventFlagMaskShift);
                }
                self.sync_key.pop();
            }
        }
    }

    unsafe fn send_empty_character(&mut self, code_table: usize) {
        if is_double_code(code_table) {
            self.sync_key.push(1);
        }
        let ch: u16 = if self.front_app.starts_with("com.sublimetext.") { 0x200C } else { 0x202F };
        unsafe { self.post_unicode(&[ch]) };
    }

    /// Converts one engine output value into UTF-16 code units, tracking lengths for 2-byte code tables
    fn encode_char(&mut self, data: u32, code_table: usize, out: &mut Vec<u16>) {
        if (data & PURE_CHARACTER_MASK) != 0 {
            out.push(data as u16);
            if is_double_code(code_table) {
                self.sync_key.push(1);
            }
        } else if (data & CHAR_CODE_MASK) == 0 {
            if is_double_code(code_table) {
                self.sync_key.push(1);
            }
            let ch = key_code_to_character(data);
            if ch != 0 {
                out.push(ch);
            }
        } else {
            let raw = data as u16;
            match code_table {
                0 => out.push(raw),
                1 | 2 | 4 => {
                    let lo = raw & 0xFF;
                    let hi = raw >> 8;
                    out.push(lo);
                    if hi > 32 {
                        if code_table == 2 {
                            self.sync_key.push(2);
                        }
                        out.push(hi);
                    } else if code_table == 2 {
                        self.sync_key.push(1);
                    }
                }
                3 => {
                    let hi = (raw >> 13) as usize;
                    out.push(raw & 0x1FFF);
                    if hi > 0 && hi <= UNICODE_COMPOUND_MARK.len() {
                        self.sync_key.push(2);
                        out.push(UNICODE_COMPOUND_MARK[hi - 1]);
                    } else {
                        self.sync_key.push(1);
                    }
                }
                _ => out.push(raw),
            }
        }
    }

    fn check_hot_key(&self, hotkey: u32, check_code: bool) -> bool {
        if (hotkey & !0x8000) == EMPTY_HOTKEY {
            return false;
        }
        let flag = self.last_flag;
        if ((hotkey & 0x100) != 0) != ((flag & kCGEventFlagMaskControl) != 0)
            || ((hotkey & 0x200) != 0) != ((flag & kCGEventFlagMaskAlternate) != 0)
            || ((hotkey & 0x400) != 0) != ((flag & kCGEventFlagMaskCommand) != 0)
            || ((hotkey & 0x800) != 0) != ((flag & kCGEventFlagMaskShift) != 0)
        {
            return false;
        }
        !check_code || (hotkey & 0xFF) as u16 == self.keycode
    }

    fn remember_app_status(&self, lang: u32, code_table: usize) {
        if self.front_app.is_empty() {
            return;
        }
        if let Ok(mut table) = self.smart_switch.lock() {
            table.update(&self.front_app, lang, code_table);
            crate::config::set_reg_binary("smartSwitchKey", &table.to_binary());
        }
    }

    fn switch_language(&mut self) {
        let next = if self.config.language.load(Ordering::SeqCst) == 0 { 1 } else { 0 };
        self.config.language.store(next, Ordering::SeqCst);
        if (self.config.switch_key_status.load(Ordering::Relaxed) & 0x8000) != 0 {
            crate::hook::MessageBeep(0);
        }
        let code_table = match self.engine.lock() {
            Ok(mut eng) => {
                eng.language = next;
                eng.start_new_session();
                eng.code_table
            }
            Err(_) => 0,
        };
        if self.config.use_smart_switch_key.load(Ordering::Relaxed) {
            self.remember_app_status(next, code_table);
        }
        (self.on_language_changed)(next);
    }

    /// Smart switch: restore language / code table remembered for the newly focused app
    fn on_active_app_changed(&mut self) {
        let use_smart = self.config.use_smart_switch_key.load(Ordering::Relaxed);
        let remember_code = self.config.remember_code.load(Ordering::Relaxed);
        if !use_smart && !remember_code {
            return;
        }
        let cur_lang = self.config.language.load(Ordering::Relaxed);
        let cur_table = self.engine.lock().map(|e| e.code_table).unwrap_or(0);
        let current = ((cur_lang & 0x01) as u8) | ((cur_table.min(4) as u8) << 1);

        let status = match self.smart_switch.lock() {
            Ok(mut table) => {
                let status = table.get_app_input_method_status(&self.front_app, current);
                if status < 0 {
                    crate::config::set_reg_binary("smartSwitchKey", &table.to_binary());
                }
                status
            }
            Err(_) => return,
        };
        if status < 0 {
            return;
        }

        let app_lang = (status & 0x01) as u32;
        let app_table = ((status >> 1) & 0x07) as usize;
        if use_smart && app_lang != cur_lang {
            self.config.language.store(app_lang, Ordering::SeqCst);
            if let Ok(mut eng) = self.engine.lock() {
                eng.language = app_lang;
                eng.start_new_session();
            }
            (self.on_language_changed)(app_lang);
        }
        if remember_code && app_table != cur_table {
            if let Ok(mut eng) = self.engine.lock() {
                eng.code_table = app_table;
            }
            (self.on_code_table_changed)(app_table as u32);
        }
    }

    fn refresh_front_app(&mut self) {
        if let Some(app) = crate::smart_switch::get_frontmost_app_name() {
            if app != self.front_app {
                self.front_app = app;
                self.on_active_app_changed();
            }
        }
    }

    fn request_new_session(&mut self) {
        if let Ok(mut eng) = self.engine.lock() {
            eng.handle_event(KeyEvent::Mouse, KeyEventState::MouseDown, 0, 0, false);
            if is_double_code(eng.code_table) {
                self.sync_key.clear();
            }
        }
    }

    /// Sends the result of a macro expansion (backspaces + replacement text)
    unsafe fn send_macro(&mut self, state: &HookState, code_table: usize) {
        let mut bpc = state.backspace_count as usize;
        if self.config.fix_recommend_browser.load(Ordering::Relaxed) && self.front_app != SPOTLIGHT {
            unsafe { self.send_empty_character(code_table) };
            bpc += 1;
        }
        for _ in 0..bpc {
            unsafe { self.send_backspace(code_table) };
        }
        let mut out = Vec::with_capacity(state.macro_data.len() + 4);
        for &data in &state.macro_data {
            self.encode_char(data, code_table, &mut out);
        }
        unsafe { self.post_unicode(&out) };
    }
}

unsafe extern "C" fn mac_event_tap_callback(
    proxy: CGEventTapProxy,
    event_type: u32,
    event: CGEventRef,
    _refcon: *mut c_void,
) -> CGEventRef {
    unsafe {
        let Some(ctx) = GLOBAL_MAC_HOOK.as_mut() else {
            return event;
        };

        // macOS disables taps that are too slow or when secure input toggles; turn it back on
        if event_type == kCGEventTapDisabledByTimeout || event_type == kCGEventTapDisabledByUserInput {
            if !ctx.tap.is_null() {
                CGEventTapEnable(ctx.tap, true);
            }
            return event;
        }

        // Don't handle our own synthetic events
        if CGEventGetIntegerValueField(event, kCGEventSourceUserData) == MINKEY_EVENT_MARKER {
            return event;
        }

        ctx.proxy = proxy;
        ctx.flag = CGEventGetFlags(event);
        if event_type == kCGEventKeyDown || event_type == kCGEventKeyUp {
            let mac_code = CGEventGetIntegerValueField(event, kCGKeyboardEventKeycode) as u16;
            ctx.keycode = mac_keycode_to_engine_key(mac_code);
        }

        let switch_hotkey = ctx.config.switch_key_status.load(Ordering::Relaxed);
        let convert_hotkey = ctx.config.convert_tool_hotkey.load(Ordering::Relaxed);
        let switch_code = (switch_hotkey & 0xFF) as u16;
        let convert_code = (convert_hotkey & 0xFF) as u16;

        // Language switch / quick convert hotkeys
        if event_type == kCGEventKeyDown {
            if switch_code != ctx.keycode && convert_code != ctx.keycode {
                ctx.last_flag = 0;
            } else {
                // Hotkeys use the modifiers held right now
                ctx.last_flag = ctx.flag;
                if switch_code == ctx.keycode && ctx.check_hot_key(switch_hotkey, switch_code != 0xFE) {
                    ctx.refresh_front_app();
                    ctx.switch_language();
                    ctx.last_flag = 0;
                    ctx.has_just_used_hotkey = true;
                    return std::ptr::null_mut();
                }
                if convert_code == ctx.keycode && ctx.check_hot_key(convert_hotkey, convert_code != 0xFE) {
                    (ctx.on_quick_convert)();
                    ctx.last_flag = 0;
                    ctx.has_just_used_hotkey = true;
                    return std::ptr::null_mut();
                }
            }
            ctx.has_just_used_hotkey = ctx.last_flag != 0;
        } else if event_type == kCGEventFlagsChanged {
            if ctx.last_flag == 0 || ctx.last_flag < ctx.flag {
                ctx.last_flag = ctx.flag;
            } else if ctx.last_flag > ctx.flag {
                // Modifier-only hotkeys fire when the modifiers are released
                if switch_code == 0xFE && ctx.check_hot_key(switch_hotkey, false) {
                    ctx.last_flag = 0;
                    ctx.refresh_front_app();
                    ctx.switch_language();
                    ctx.has_just_used_hotkey = true;
                    return event;
                }
                if convert_code == 0xFE && ctx.check_hot_key(convert_hotkey, false) {
                    ctx.last_flag = 0;
                    (ctx.on_quick_convert)();
                    ctx.has_just_used_hotkey = true;
                    return event;
                }
                if !ctx.has_just_used_hotkey {
                    if let Ok(mut eng) = ctx.engine.lock() {
                        if ctx.config.temp_off_spelling.load(Ordering::Relaxed)
                            && (ctx.last_flag & kCGEventFlagMaskControl) != 0
                        {
                            eng.temp_off_spell_checking();
                        }
                        if ctx.config.temp_off_openkey.load(Ordering::Relaxed)
                            && (ctx.last_flag & kCGEventFlagMaskCommand) != 0
                        {
                            let cur = eng.will_temp_off_engine;
                            eng.temp_off_engine(!cur);
                        }
                    }
                }
                ctx.last_flag = 0;
                ctx.has_just_used_hotkey = false;
            }
            return event;
        }

        let is_mouse = matches!(
            event_type,
            kCGEventLeftMouseDown
                | kCGEventRightMouseDown
                | kCGEventOtherMouseDown
                | kCGEventLeftMouseDragged
                | kCGEventRightMouseDragged
        );
        if event_type != kCGEventKeyDown && !is_mouse {
            return event;
        }

        if is_mouse {
            if event_type != kCGEventLeftMouseDragged && event_type != kCGEventRightMouseDragged {
                ctx.refresh_front_app();
            }
            ctx.request_new_session();
            return event;
        }

        ctx.refresh_front_app();

        let is_shift = (ctx.flag & kCGEventFlagMaskShift) != 0;
        let is_caps_lock = (ctx.flag & kCGEventFlagMaskAlphaShift) != 0;
        let other_control = (ctx.flag & OTHER_CONTROL_MASK) != 0;

        // English mode: only macros (if enabled)
        if ctx.config.language.load(Ordering::Relaxed) == 0 {
            if ctx.config.use_macro.load(Ordering::Relaxed) && ctx.config.use_macro_in_english.load(Ordering::Relaxed) {
                let (state, code_table) = {
                    let Ok(mut eng) = ctx.engine.lock() else { return event };
                    eng.handle_english_mode(ctx.keycode, is_shift || is_caps_lock, other_control);
                    (eng.hook_state.clone(), eng.code_table)
                };
                if state.code == HookCodeState::ReplaceMacro {
                    ctx.send_macro(&state, code_table);
                    ctx.repost_original(event);
                    return std::ptr::null_mut();
                }
            }
            return event;
        }

        let caps_status: u8 = if is_shift { 1 } else if is_caps_lock { 2 } else { 0 };
        let (state, code_table) = {
            let Ok(mut eng) = ctx.engine.lock() else { return event };
            let state = eng
                .handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, ctx.keycode, caps_status, other_control)
                .clone();
            (state, eng.code_table)
        };

        match state.code {
            HookCodeState::DoNothing | HookCodeState::BreakWord => {
                if is_double_code(code_table) {
                    match state.ext_code {
                        1 => ctx.sync_key.clear(),
                        2 => {
                            if let Some(len) = ctx.sync_key.pop() {
                                if len > 1 && (code_table == 2 || !is_unicode_compound_app(&ctx.front_app)) {
                                    ctx.post_key(MAC_KEY_DELETE, 0);
                                }
                            }
                        }
                        3 => ctx.sync_key.push(1),
                        _ => {}
                    }
                }
                event
            }
            HookCodeState::WillProcess | HookCodeState::Restore | HookCodeState::RestoreAndStartNewSession => {
                let mut bpc = state.backspace_count as usize;

                // Fix browser address bar autocomplete eating the first backspace
                if ctx.config.fix_recommend_browser.load(Ordering::Relaxed)
                    && ctx.front_app != SPOTLIGHT
                    && state.ext_code != 4
                {
                    if ctx.config.fix_chromium_browser.load(Ordering::Relaxed) && is_chromium_app(&ctx.front_app) {
                        if bpc > 0 {
                            ctx.send_shift_left(code_table);
                            if bpc == 1 {
                                bpc = 0;
                            }
                        }
                    } else {
                        ctx.send_empty_character(code_table);
                        bpc += 1;
                    }
                }

                // Spotlight ignores synthetic backspaces reliably only as a selection replacement
                if ctx.front_app == SPOTLIGHT && bpc > 0 {
                    for _ in 0..bpc {
                        ctx.send_shift_left(code_table);
                    }
                    bpc = 0;
                }

                if bpc < MAX_BUFF {
                    for _ in 0..bpc {
                        ctx.send_backspace(code_table);
                    }
                }

                let count = (state.new_char_count as usize).min(MAX_BUFF);
                let mut out = Vec::with_capacity(count * 2 + 1);
                for i in (0..count).rev() {
                    let data = state.char_data[i];
                    ctx.encode_char(data, code_table, &mut out);
                }
                ctx.post_unicode(&out);

                if state.code != HookCodeState::WillProcess {
                    // Invalid word: re-type the key the user pressed as-is
                    if is_double_code(code_table) {
                        ctx.sync_key.push(1);
                    }
                    ctx.repost_original(event);
                    if state.code == HookCodeState::RestoreAndStartNewSession {
                        if let Ok(mut eng) = ctx.engine.lock() {
                            eng.start_new_session();
                        }
                    }
                }
                std::ptr::null_mut()
            }
            HookCodeState::ReplaceMacro => {
                ctx.send_macro(&state, code_table);
                ctx.repost_original(event);
                std::ptr::null_mut()
            }
        }
    }
}
