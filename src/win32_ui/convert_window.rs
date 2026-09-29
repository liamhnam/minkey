// Encoding conversion tool: source / target code table, text options, clipboard hotkey

use std::cell::Cell;
use std::sync::atomic::Ordering;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::UI::Controls::EM_SETLIMITTEXT;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{PCWSTR, w};

use super::controls::{self, Form};
use super::ctx;
use crate::settings::{self, Hotkey};

pub const CLASS: PCWSTR = w!("MinkeyConvertWnd");
const CLIENT: (i32, i32) = (460, 402);

const ID_FROM: i32 = 401;
const ID_TO: i32 = 402;
const ID_SWAP: i32 = 403;
/// Check box of `OPTIONS[i]` has id `ID_OPTION + i`
const ID_OPTION: i32 = 410;
const ID_HK_CTRL: i32 = 420;
const ID_HK_ALT: i32 = 421;
const ID_HK_WIN: i32 = 422;
const ID_HK_SHIFT: i32 = 423;
const ID_HK_KEY: i32 = 424;
const ID_CONVERT: i32 = 430;
const ID_CLOSE: i32 = 431;

const CODE_TABLES: [&str; 5] = ["Unicode dựng sẵn", "TCVN3 (ABC)", "VNI Windows", "Unicode tổ hợp", "Vietnamese locale CP1258"];

const OPTIONS: [(&str, &str); 6] = [
    ("to_all_caps", "Chuyển sang chữ HOA"),
    ("to_all_non_caps", "Chuyển sang chữ thường"),
    ("to_caps_each_word", "Viết hoa chữ đầu mỗi từ"),
    ("to_caps_first_letter", "Viết hoa chữ đầu câu"),
    ("remove_mark", "Loại bỏ dấu tiếng Việt"),
    ("dont_alert", "Không hiện thông báo sau khi chuyển xong"),
];

thread_local! {
    static HWND_SELF: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    static POPULATING: Cell<bool> = const { Cell::new(false) };
}

pub fn hwnd() -> HWND {
    HWND_SELF.with(|h| h.get())
}

pub fn show() {
    let mut hwnd = hwnd();
    if hwnd.is_null() {
        hwnd = controls::create_panel(CLASS, "Minkey - Công cụ chuyển mã", CLIENT);
        if hwnd.is_null() {
            return;
        }
        HWND_SELF.with(|h| h.set(hwnd));
        build(hwnd);
    }
    refresh(hwnd);
    controls::bring_to_front(hwnd);
}

fn build(hwnd: HWND) {
    let f = Form::new(hwnd);
    f.group("Bảng mã chuyển đổi", (12, 8, 436, 90));
    f.label("Nguồn:", (24, 32, 180, 20));
    f.combo(ID_FROM, &CODE_TABLES, (24, 54, 180, 200));
    f.button(ID_SWAP, "⇄", (212, 53, 36, 26), false);
    f.label("Đích:", (256, 32, 180, 20));
    f.combo(ID_TO, &CODE_TABLES, (256, 54, 180, 200));

    f.group("Tùy chọn biến đổi văn bản", (12, 106, 436, 176));
    for (i, (_, label)) in OPTIONS.iter().enumerate() {
        f.check(ID_OPTION + i as i32, label, (24, 130 + 24 * i as i32, 410, 20));
    }

    f.group("Phím tắt chuyển nhanh Clipboard", (12, 290, 436, 58));
    f.check(ID_HK_CTRL, "Ctrl", (24, 316, 52, 20));
    f.check(ID_HK_ALT, "Alt", (82, 316, 48, 20));
    f.check(ID_HK_WIN, "Win", (136, 316, 50, 20));
    f.check(ID_HK_SHIFT, "Shift", (192, 316, 56, 20));
    f.label("+", (254, 318, 12, 20));
    let key = f.edit(ID_HK_KEY, (270, 314, 36, 23), ES_UPPERCASE as u32 | ES_CENTER as u32);
    unsafe { SendMessageW(key, EM_SETLIMITTEXT, 1, 0) };

    f.button(ID_CONVERT, "Chuyển mã Clipboard", (12, 360, 180, 30), true);
    f.button(ID_CLOSE, "Đóng", (348, 360, 100, 30), false);
}

fn refresh(hwnd: HWND) {
    let ctx = ctx();
    let config = &ctx.config;
    let opts = settings::convert_options(config);
    let values = [
        opts.to_all_caps,
        opts.to_all_non_caps,
        opts.to_caps_each_word,
        opts.to_caps_first_letter,
        opts.remove_mark,
        opts.dont_alert,
    ];
    let item = |id| controls::item(hwnd, id);
    POPULATING.with(|p| p.set(true));
    controls::set_combo_index(item(ID_FROM), opts.from_code as i32);
    controls::set_combo_index(item(ID_TO), opts.to_code as i32);
    for (i, value) in values.iter().enumerate() {
        controls::set_checked(item(ID_OPTION + i as i32), *value);
    }
    let hk = settings::decode_hotkey(config.convert_hotkey.load(Ordering::Relaxed));
    controls::set_checked(item(ID_HK_CTRL), hk.ctrl);
    controls::set_checked(item(ID_HK_ALT), hk.alt);
    controls::set_checked(item(ID_HK_WIN), hk.win);
    controls::set_checked(item(ID_HK_SHIFT), hk.shift);
    controls::set_text(item(ID_HK_KEY), &hk.key.map(String::from).unwrap_or_default());
    POPULATING.with(|p| p.set(false));
}

fn selected_codes(hwnd: HWND) -> (u32, u32) {
    let index = |id| controls::combo_index(controls::item(hwnd, id)).max(0) as u32;
    (index(ID_FROM), index(ID_TO))
}

fn on_command(hwnd: HWND, id: i32, code: u32) {
    if POPULATING.with(|p| p.get()) {
        return;
    }
    let ctx = ctx();
    let item = |id| controls::item(hwnd, id);
    match (id, code) {
        (ID_FROM | ID_TO, CBN_SELCHANGE) => {
            let (from, to) = selected_codes(hwnd);
            settings::set_convert_codes(&ctx.config, from, to);
        }
        (ID_SWAP, BN_CLICKED) => {
            let (from, to) = selected_codes(hwnd);
            settings::set_convert_codes(&ctx.config, to, from);
            refresh(hwnd);
        }
        (ID_HK_CTRL..=ID_HK_SHIFT, BN_CLICKED) | (ID_HK_KEY, EN_CHANGE) => {
            let hotkey = Hotkey {
                ctrl: controls::is_checked(item(ID_HK_CTRL)),
                alt: controls::is_checked(item(ID_HK_ALT)),
                win: controls::is_checked(item(ID_HK_WIN)),
                shift: controls::is_checked(item(ID_HK_SHIFT)),
                key: controls::get_text(item(ID_HK_KEY)).chars().next(),
                beep: false,
            };
            settings::set_convert_hotkey(&ctx.config, &hotkey);
        }
        (ID_CONVERT, BN_CLICKED) | (IDOK, _) => settings::convert_clipboard(&ctx.config),
        (ID_CLOSE, BN_CLICKED) | (IDCANCEL, _) => unsafe {
            DestroyWindow(hwnd);
        },
        _ if (ID_OPTION..ID_OPTION + OPTIONS.len() as i32).contains(&id) && code == BN_CLICKED => {
            let (name, _) = OPTIONS[(id - ID_OPTION) as usize];
            settings::set_convert_option(&ctx.config, name, controls::is_checked(item(id)));
            // The letter-case options exclude each other
            refresh(hwnd);
        }
        _ => {}
    }
}

pub unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_COMMAND => {
            on_command(hwnd, (wparam & 0xFFFF) as i32, ((wparam >> 16) & 0xFFFF) as u32);
            0
        }
        WM_CTLCOLORSTATIC => controls::ctl_color(wparam as HDC, lparam as HWND),
        WM_DPICHANGED => {
            controls::on_dpi_changed(hwnd, wparam, lparam);
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            HWND_SELF.with(|h| h.set(std::ptr::null_mut()));
            super::panel_closed();
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
