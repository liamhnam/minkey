// System Tray Service for Minkey
// Ported from OpenKey SystemTrayHelper.cpp

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};
#[cfg(windows)]
use std::sync::{Arc, Mutex};
#[cfg(windows)]
use std::thread;

#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::Graphics::Gdi::*;
#[cfg(windows)]
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
#[cfg(windows)]
use windows_sys::Win32::UI::Shell::*;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(windows)]
use crate::hook::HookConfig;
#[cfg(windows)]
use crate::types::{CodeTable, InputType};
#[cfg(windows)]
use crate::VietnameseEngine;

pub const WM_TRAYMESSAGE: u32 = 0x8000 + 1; // WM_USER + 1
pub const TRAY_ICON_UID: u32 = 100;


// Menu Command IDs matching OpenKey
pub const POPUP_VIET_ON_OFF: u32 = 900;
pub const POPUP_SPELLING: u32 = 901;
pub const POPUP_SMART_SWITCH: u32 = 902;
pub const POPUP_USE_MACRO: u32 = 903;

pub const POPUP_TELEX: u32 = 910;
pub const POPUP_VNI: u32 = 911;
pub const POPUP_SIMPLE_TELEX: u32 = 912;

pub const POPUP_UNICODE: u32 = 930;
pub const POPUP_TCVN3: u32 = 931;
pub const POPUP_VNI_WINDOWS: u32 = 932;
pub const POPUP_UNICODE_COMPOUND: u32 = 933;
pub const POPUP_VN_LOCALE_1258: u32 = 934;

pub const POPUP_CONVERT_TOOL: u32 = 980;
pub const POPUP_QUICK_CONVERT: u32 = 981;
pub const POPUP_MACRO_TABLE: u32 = 990;

pub const POPUP_CONTROL_PANEL: u32 = 1000;
pub const POPUP_ABOUT_MINKEY: u32 = 1010;
pub const POPUP_MINKEY_EXIT: u32 = 2000;

pub struct TrayCallbacks {
    pub on_open_control_panel: Box<dyn Fn() + Send + Sync>,
    pub on_open_macro_table: Box<dyn Fn() + Send + Sync>,
    pub on_open_convert_tool: Box<dyn Fn() + Send + Sync>,
    pub on_open_about: Box<dyn Fn() + Send + Sync>,
    pub on_exit: Box<dyn Fn() + Send + Sync>,
}

#[cfg(windows)]
pub type PlatformTrayService = TrayService;

#[cfg(not(windows))]
pub use crate::tray_macos::MacTrayService;

#[cfg(not(windows))]
pub type PlatformTrayService = MacTrayService;

#[cfg(not(windows))]
pub type TrayService = MacTrayService;

/// Asks the tray / menu bar icon to redraw after language or icon style changed.
/// Safe to call from any thread.
pub fn refresh_tray(tray_hwnd: usize) {
    #[cfg(windows)]
    if tray_hwnd != 0 {
        unsafe {
            PostMessageW(tray_hwnd as HWND, WM_USER + 2026, 0, 0);
        }
    }
    #[cfg(not(windows))]
    {
        let _ = tray_hwnd;
        crate::tray_macos::request_refresh();
    }
}

#[cfg(windows)]
pub struct TrayService {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    callbacks: Arc<TrayCallbacks>,
    thread_handle: Option<thread::JoinHandle<()>>,
    hwnd: usize,
}

#[cfg(windows)]
static mut GLOBAL_TRAY: *mut TrayContext = std::ptr::null_mut();

#[cfg(windows)]
struct TrayContext {
    config: HookConfig,
    engine: Arc<Mutex<VietnameseEngine>>,
    callbacks: Arc<TrayCallbacks>,
    hwnd: HWND,
    nid: NOTIFYICONDATAW,
    popup_menu: HMENU,
    other_code_menu: HMENU,
    taskbar_created_msg: u32,
    use_gray_icon: Arc<AtomicBool>,
}

#[cfg(windows)]
impl TrayService {

    pub fn new(
        config: HookConfig,
        engine: Arc<Mutex<VietnameseEngine>>,
        callbacks: TrayCallbacks,
    ) -> Self {
        Self {
            config,
            engine,
            callbacks: Arc::new(callbacks),
            thread_handle: None,
            hwnd: 0,
        }
    }

    pub fn start(&mut self, use_gray_icon: Arc<AtomicBool>) {
        let config = self.config.clone();
        let engine = self.engine.clone();
        let callbacks = self.callbacks.clone();

        let (tx, rx) = std::sync::mpsc::channel();

        let handle = thread::spawn(move || {
            unsafe {
                let h_instance = GetModuleHandleW(std::ptr::null());
                let class_name = wide_str("MinkeyTrayWndClass");

                let mut wc: WNDCLASSW = std::mem::zeroed();
                wc.lpfnWndProc = Some(tray_wnd_proc);
                wc.hInstance = h_instance;
                wc.lpszClassName = class_name.as_ptr();

                RegisterClassW(&wc);

                let hwnd = CreateWindowExW(
                    0,
                    class_name.as_ptr(),
                    wide_str("MinkeyTrayWindow").as_ptr(),
                    0,
                    0, 0, 0, 0,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    h_instance,
                    std::ptr::null(),
                );

                let taskbar_created = RegisterWindowMessageW(wide_str("TaskbarCreated").as_ptr());

                let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
                nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
                nid.hWnd = hwnd;
                nid.uID = TRAY_ICON_UID;
                nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
                nid.uCallbackMessage = WM_TRAYMESSAGE;

                let is_viet = config.language.load(Ordering::Relaxed) != 0;
                let gray = use_gray_icon.load(Ordering::Relaxed);
                nid.hIcon = create_tray_icon(is_viet, gray);
                copy_tip(&mut nid.szTip, if is_viet { "Minkey - Tiếng Việt" } else { "Minkey - English" });

                Shell_NotifyIconW(NIM_ADD, &nid);

                let (popup_menu, other_code_menu) = create_tray_popup_menu();

                let ctx = Box::new(TrayContext {
                    config,
                    engine,
                    callbacks,
                    hwnd,
                    nid,
                    popup_menu,
                    other_code_menu,
                    taskbar_created_msg: taskbar_created,
                    use_gray_icon,
                });
                GLOBAL_TRAY = Box::into_raw(ctx);

                let _ = tx.send(hwnd as usize);

                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }

                if !GLOBAL_TRAY.is_null() {
                    let ctx = Box::from_raw(GLOBAL_TRAY);
                    GLOBAL_TRAY = std::ptr::null_mut();
                    Shell_NotifyIconW(NIM_DELETE, &ctx.nid);
                    if !ctx.nid.hIcon.is_null() {
                        DestroyIcon(ctx.nid.hIcon);
                    }
                    DestroyMenu(ctx.popup_menu);
                }
            }
        });

        self.hwnd = rx.recv().unwrap_or(0);
        self.thread_handle = Some(handle);
    }

    pub fn get_hwnd(&self) -> usize {
        self.hwnd
    }

    pub fn update_icon(&self) {
        unsafe {
            if !GLOBAL_TRAY.is_null() {
                let ctx = &mut *GLOBAL_TRAY;
                PostMessageW(ctx.hwnd, WM_USER + 2026, 0, 0);
            }
        }
    }

    pub fn stop(&mut self) {
        if self.hwnd != 0 {
            unsafe {
                PostMessageW(self.hwnd as HWND, WM_CLOSE, 0, 0);
            }
        }
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

// Window procedure for tray message-handling window
#[cfg(windows)]
unsafe extern "system" fn tray_wnd_proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    unsafe {
        if !GLOBAL_TRAY.is_null() {
            let ctx = &mut *GLOBAL_TRAY;

            if msg == ctx.taskbar_created_msg {
                // Taskbar was restarted by Explorer -> re-add tray icon!
                Shell_NotifyIconW(NIM_ADD, &ctx.nid);
                return 0;
            }

            if msg == WM_USER + 2026 {
                // Internal request to update icon
                update_tray_data(ctx);
                return 0;
            }

            if msg == WM_TRAYMESSAGE {
                let event = l_param as u32;
                if event == WM_LBUTTONDBLCLK {
                    (ctx.callbacks.on_open_control_panel)();
                    return 0;
                } else if event == WM_LBUTTONUP {
                    // Toggle Vietnamese / English
                    let cur = ctx.config.language.load(Ordering::SeqCst);
                    let next = if cur == 0 { 1 } else { 0 };
                    ctx.config.language.store(next, Ordering::SeqCst);
                    if let Ok(mut engine) = ctx.engine.lock() {
                        engine.language = next;
                        engine.start_new_session();
                    }
                    update_tray_data(ctx);
                    return 0;
                } else if event == WM_RBUTTONDOWN || event == WM_RBUTTONUP {
                    if event == WM_RBUTTONUP {
                        show_tray_menu(ctx);
                    }
                    return 0;
                }
            }
        }

        match msg {
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, w_param, l_param),
        }
    }
}

#[cfg(windows)]
unsafe fn show_tray_menu(ctx: &mut TrayContext) {
    unsafe {
        let mut pt: POINT = std::mem::zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(ctx.hwnd);

        // Sync menu state with current settings
        let is_viet = ctx.config.language.load(Ordering::Relaxed) != 0;
        let (input_type, code_table, check_spelling, use_macro, smart_switch) = {
            if let Ok(engine) = ctx.engine.lock() {
                (
                    engine.input_type,
                    engine.code_table,
                    engine.check_spelling,
                    engine.use_macro,
                    ctx.config.use_smart_switch_key.load(Ordering::Relaxed),
                )
            } else {
                (InputType::Telex, 0, true, true, true)
            }
        };

        check_menu_item(ctx.popup_menu, POPUP_VIET_ON_OFF, is_viet);
        check_menu_item(ctx.popup_menu, POPUP_SPELLING, check_spelling);
        check_menu_item(ctx.popup_menu, POPUP_SMART_SWITCH, smart_switch);
        check_menu_item(ctx.popup_menu, POPUP_USE_MACRO, use_macro);

        check_menu_item(ctx.popup_menu, POPUP_TELEX, input_type == InputType::Telex);
        check_menu_item(ctx.popup_menu, POPUP_VNI, input_type == InputType::Vni);
        check_menu_item(ctx.popup_menu, POPUP_SIMPLE_TELEX, input_type == InputType::SimpleTelex1 || input_type == InputType::SimpleTelex2);

        check_menu_item(ctx.popup_menu, POPUP_UNICODE, code_table == 0);
        check_menu_item(ctx.popup_menu, POPUP_TCVN3, code_table == 1);
        check_menu_item(ctx.popup_menu, POPUP_VNI_WINDOWS, code_table == 2);
        check_menu_item(ctx.other_code_menu, POPUP_UNICODE_COMPOUND, code_table == 3);
        check_menu_item(ctx.other_code_menu, POPUP_VN_LOCALE_1258, code_table == 4);

        let cmd = TrackPopupMenu(
            ctx.popup_menu,
            TPM_RETURNCMD | TPM_NONOTIFY,
            pt.x,
            pt.y,
            0,
            ctx.hwnd,
            std::ptr::null(),
        ) as u32;

        match cmd {
            POPUP_VIET_ON_OFF => {
                let next = if is_viet { 0 } else { 1 };
                ctx.config.language.store(next, Ordering::SeqCst);
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.language = next;
                    engine.start_new_session();
                }
                update_tray_data(ctx);
            }
            POPUP_SPELLING => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    let cur = engine.check_spelling;
                    engine.set_check_spelling(!cur);
                }
                update_tray_data(ctx);
            }
            POPUP_SMART_SWITCH => {
                let cur = ctx.config.use_smart_switch_key.load(Ordering::Relaxed);
                ctx.config.use_smart_switch_key.store(!cur, Ordering::Relaxed);
            }
            POPUP_USE_MACRO => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.use_macro = !engine.use_macro;
                }
            }
            POPUP_TELEX => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.input_type = InputType::Telex;
                    engine.start_new_session();
                }
                update_tray_data(ctx);
            }
            POPUP_VNI => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.input_type = InputType::Vni;
                    engine.start_new_session();
                }
                update_tray_data(ctx);
            }
            POPUP_SIMPLE_TELEX => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.input_type = InputType::SimpleTelex1;
                    engine.start_new_session();
                }
                update_tray_data(ctx);
            }
            POPUP_UNICODE => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.code_table = CodeTable::Unicode.to_u32() as usize;
                }
                update_tray_data(ctx);
            }
            POPUP_TCVN3 => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.code_table = CodeTable::Tcvn3.to_u32() as usize;
                }
                update_tray_data(ctx);
            }
            POPUP_VNI_WINDOWS => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.code_table = CodeTable::VniWindows.to_u32() as usize;
                }
                update_tray_data(ctx);
            }
            POPUP_UNICODE_COMPOUND => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.code_table = CodeTable::UnicodeCompound.to_u32() as usize;
                }
                update_tray_data(ctx);
            }
            POPUP_VN_LOCALE_1258 => {
                if let Ok(mut engine) = ctx.engine.lock() {
                    engine.code_table = CodeTable::Cp1258.to_u32() as usize;
                }
                update_tray_data(ctx);
            }
            POPUP_CONTROL_PANEL => {
                (ctx.callbacks.on_open_control_panel)();
            }
            POPUP_MACRO_TABLE => {
                (ctx.callbacks.on_open_macro_table)();
            }
            POPUP_CONVERT_TOOL => {
                (ctx.callbacks.on_open_convert_tool)();
            }
            POPUP_ABOUT_MINKEY => {
                (ctx.callbacks.on_open_about)();
            }
            POPUP_MINKEY_EXIT => {
                (ctx.callbacks.on_exit)();
            }
            _ => {}
        }
    }
}

#[cfg(windows)]
unsafe fn update_tray_data(ctx: &mut TrayContext) {
    unsafe {
        let is_viet = ctx.config.language.load(Ordering::Relaxed) != 0;
        let gray = ctx.use_gray_icon.load(Ordering::Relaxed);

        if !ctx.nid.hIcon.is_null() {
            DestroyIcon(ctx.nid.hIcon);
        }
        ctx.nid.hIcon = create_tray_icon(is_viet, gray);
        copy_tip(&mut ctx.nid.szTip, if is_viet { "Minkey - Tiếng Việt" } else { "Minkey - English" });

        Shell_NotifyIconW(NIM_MODIFY, &ctx.nid);
    }
}

#[cfg(windows)]
unsafe fn create_tray_popup_menu() -> (HMENU, HMENU) {
    unsafe {
        let popup = CreatePopupMenu();
        append_menu_item(popup, MF_STRING, POPUP_VIET_ON_OFF, "Bật Tiếng Việt");
        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_SPELLING, "Bật kiểm tra chính tả");
        append_menu_item(popup, MF_STRING, POPUP_SMART_SWITCH, "Bật loại trừ ứng dụng thông minh");
        append_menu_item(popup, MF_STRING, POPUP_USE_MACRO, "Bật gõ tắt");
        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_MACRO_TABLE, "Cấu hình gõ tắt...");
        append_menu_item(popup, MF_STRING, POPUP_CONVERT_TOOL, "Công cụ chuyển mã...");
        append_menu_item(popup, MF_STRING, POPUP_QUICK_CONVERT, "Chuyển mã nhanh");
        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_TELEX, "Kiểu gõ Telex");
        append_menu_item(popup, MF_STRING, POPUP_VNI, "Kiểu gõ VNI");
        append_menu_item(popup, MF_STRING, POPUP_SIMPLE_TELEX, "Kiểu gõ Simple Telex");
        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_UNICODE, "Unicode dựng sẵn");
        append_menu_item(popup, MF_STRING, POPUP_TCVN3, "TCVN3 (ABC)");
        append_menu_item(popup, MF_STRING, POPUP_VNI_WINDOWS, "VNI Windows");

        let other_code = CreatePopupMenu();
        append_menu_item(other_code, MF_STRING, POPUP_UNICODE_COMPOUND, "Unicode tổ hợp");
        append_menu_item(other_code, MF_STRING, POPUP_VN_LOCALE_1258, "Vietnamese locale CP 1258");
        AppendMenuW(popup, MF_POPUP, other_code as usize, wide_str("Bảng mã khác").as_ptr());

        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_CONTROL_PANEL, "Bảng điều khiển...");
        append_menu_item(popup, MF_STRING, POPUP_ABOUT_MINKEY, "Giới thiệu Minkey");
        AppendMenuW(popup, MF_SEPARATOR, 0, std::ptr::null());
        append_menu_item(popup, MF_STRING, POPUP_MINKEY_EXIT, "Thoát");

        SetMenuDefaultItem(popup, POPUP_CONTROL_PANEL, 0);

        (popup, other_code)
    }
}

#[cfg(windows)]
unsafe fn append_menu_item(hmenu: HMENU, flags: u32, id: u32, text: &str) {
    unsafe {
        let w = wide_str(text);
        AppendMenuW(hmenu, flags, id as usize, w.as_ptr());
    }
}

#[cfg(windows)]
unsafe fn check_menu_item(hmenu: HMENU, id: u32, checked: bool) {
    unsafe {
        CheckMenuItem(
            hmenu,
            id,
            MF_BYCOMMAND | if checked { MF_CHECKED } else { MF_UNCHECKED },
        );
    }
}

// Generate modern V / E icon dynamically in memory using GDI
#[cfg(windows)]
pub unsafe fn create_tray_icon(is_vietnamese: bool, is_gray: bool) -> HICON {
    unsafe {
        let size: i32 = 32;
        let hdc_screen = GetDC(std::ptr::null_mut());
        let hdc_mem = CreateCompatibleDC(hdc_screen);
        let hdc_mask = CreateCompatibleDC(hdc_screen);

        let hbm_color = CreateCompatibleBitmap(hdc_screen, size, size);
        let hbm_mask = CreateBitmap(size, size, 1, 1, std::ptr::null());

        let old_bmp_color = SelectObject(hdc_mem, hbm_color);
        let old_bmp_mask = SelectObject(hdc_mask, hbm_mask);

        // Background color:
        // Vietnamese: Royal Red/Crimson (#D32F2F) or Royal Blue (#1976D2); Gray mode: slate (#455A64)
        // English: Navy Slate (#37474F); Gray mode: light slate (#78909C)
        let bg_color = if is_gray {
            if is_vietnamese { 0x004A4A4A } else { 0x008A8A8A }
        } else {
            if is_vietnamese { 0x002F2FD3 } else { 0x00D27619 } // BGR: Red for V, Blue for E
        };

        let brush = CreateSolidBrush(bg_color);
        let rect = RECT { left: 0, top: 0, right: size, bottom: size };
        FillRect(hdc_mem, &rect, brush);
        DeleteObject(brush);

        // Clear mask (black = opaque for CreateIconIndirect)
        let mask_brush = CreateSolidBrush(0x00000000);
        FillRect(hdc_mask, &rect, mask_brush);
        DeleteObject(mask_brush);

        // Draw bold letter 'V' or 'E' in white
        SetBkMode(hdc_mem, TRANSPARENT as i32);
        SetTextColor(hdc_mem, 0x00FFFFFF); // White text

        let font = CreateFontW(
            24, 0, 0, 0,
            FW_BOLD as i32,
            0, 0, 0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            wide_str("Segoe UI").as_ptr(),
        );
        let old_font = SelectObject(hdc_mem, font);

        let letter = wide_str(if is_vietnamese { "V" } else { "E" });
        let mut draw_rect = RECT { left: 0, top: 2, right: size, bottom: size };
        DrawTextW(
            hdc_mem,
            letter.as_ptr(),
            1,
            &mut draw_rect,
            DT_CENTER | DT_VCENTER | DT_SINGLELINE,
        );

        SelectObject(hdc_mem, old_font);
        DeleteObject(font);

        SelectObject(hdc_mem, old_bmp_color);
        SelectObject(hdc_mask, old_bmp_mask);

        DeleteDC(hdc_mem);
        DeleteDC(hdc_mask);
        ReleaseDC(std::ptr::null_mut(), hdc_screen);

        let mut icon_info: ICONINFO = std::mem::zeroed();
        icon_info.fIcon = 1;
        icon_info.hbmColor = hbm_color;
        icon_info.hbmMask = hbm_mask;

        let hicon = CreateIconIndirect(&icon_info);

        DeleteObject(hbm_color);
        DeleteObject(hbm_mask);

        hicon
    }
}

#[cfg(windows)]
fn wide_str(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn copy_tip(dest: &mut [u16; 128], s: &str) {
    let w = wide_str(s);
    let len = w.len().min(127);
    dest[..len].copy_from_slice(&w[..len]);
    dest[len] = 0;
}
