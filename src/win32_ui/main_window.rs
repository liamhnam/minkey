// Control panel: input method, switch hotkey and every on/off option, in four tabs

use std::cell::Cell;
use std::sync::atomic::Ordering;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::{HDC, InvalidateRect};
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{PCWSTR, w};

use super::controls::{self, FontKind, Form};
use super::ctx;
use crate::settings::{self, AfterSetting, Hotkey};

pub const CLASS: PCWSTR = w!("MinkeyMainWnd");
pub const ABOUT_TAB: usize = 3;
const CLIENT: (i32, i32) = (560, 450);

const ID_LANG_VI: i32 = 101;
const ID_LANG_EN: i32 = 102;
const ID_TAB: i32 = 103;
const ID_INPUT_TYPE: i32 = 110;
const ID_CODE_TABLE: i32 = 111;
const ID_SW_CTRL: i32 = 120;
const ID_SW_ALT: i32 = 121;
const ID_SW_WIN: i32 = 122;
const ID_SW_SHIFT: i32 = 123;
const ID_SW_KEY: i32 = 124;
const ID_SW_BEEP: i32 = 125;
const ID_OPEN_MACRO: i32 = 130;
const ID_OPEN_CONVERT: i32 = 131;
const ID_GITHUB: i32 = 132;
const ID_CHECK_UPDATE: i32 = 133;
const ID_RESET: i32 = 140;
const ID_HIDE: i32 = 141;
const ID_EXIT: i32 = 142;
/// Check box of `OPTIONS[i]` has id `ID_OPTION + i`
const ID_OPTION: i32 = 200;

const TABS: [&str; 4] = ["Bộ gõ", "Gõ tắt", "Hệ thống", "Thông tin"];
const INPUT_TYPES: [&str; 4] = ["Telex", "VNI", "Simple Telex 1", "Simple Telex 2"];
const CODE_TABLES: [&str; 5] = ["Unicode dựng sẵn", "TCVN3 (ABC)", "VNI Windows", "Unicode tổ hợp", "Vietnamese locale CP1258"];

/// (setting name, label, tab page, x, y) — positions in 96-DPI units
const OPTIONS: &[(&str, &str, u8, i32, i32)] = &[
    // Bộ gõ — "Tùy chọn gõ"
    ("modern_orthography", "Đặt dấu kiểu mới (oà, uý)", 1, 34, 228),
    ("fix_browser", "Sửa lỗi gợi ý trên trình duyệt", 1, 34, 252),
    ("check_spelling", "Kiểm tra chính tả", 1, 34, 276),
    ("restore_spelling", "Khôi phục từ sai khi nhấn dấu cách", 1, 34, 300),
    ("allow_zfwj", "Cho phép phụ âm Z, W, J, F đầu từ", 1, 34, 324),
    ("temp_off_spelling", "Tạm tắt chính tả bằng nhấn đúp Ctrl", 1, 34, 348),
    ("smart_switch", "Smart Switch — nhớ chế độ theo app", 1, 290, 228),
    ("remember_code", "Tự nhớ bảng mã theo từng ứng dụng", 1, 290, 252),
    ("upper_first", "Tự viết hoa chữ đầu câu", 1, 290, 276),
    ("other_language", "Hỗ trợ gõ song ngữ", 1, 290, 300),
    ("temp_off_openkey", "Tạm tắt bộ gõ bằng phím Alt", 1, 290, 324),
    // Gõ tắt
    ("use_macro", "Bật tính năng gõ tắt (Macro)", 2, 34, 98),
    ("macro_in_english", "Cho phép gõ tắt khi đang gõ tiếng Anh", 2, 34, 122),
    ("macro_auto_caps", "Tự đổi hoa/thường theo từ viết tắt", 2, 34, 146),
    ("quick_telex", "Gõ nhanh Telex (cc→ch, gg→gi, kk→kh, nn→ng, qq→qu, pp→ph, tt→th)", 2, 34, 210),
    ("quick_start_consonant", "Phụ âm đầu thông minh (f→ph, j→gi, w→qu)", 2, 34, 234),
    ("quick_end_consonant", "Phụ âm cuối thông minh (g→ng, h→nh, k→ch)", 2, 34, 258),
    // Hệ thống
    ("run_with_windows", "Khởi động cùng Windows", 3, 34, 98),
    ("show_on_startup", "Hiện bảng điều khiển lúc khởi động", 3, 34, 122),
    ("modern_icon", "Biểu tượng màu xám (hiện đại)", 3, 34, 146),
    ("desktop_shortcut", "Tạo icon tắt nhanh ngoài Desktop", 3, 34, 170),
    ("run_as_admin", "Chạy với quyền Admin", 3, 34, 234),
    ("fix_chromium", "Bản vá đặc biệt cho trình duyệt Chromium", 3, 34, 258),
    ("use_clipboard", "Dùng Clipboard thay vì SendInput", 3, 34, 282),
    ("support_metro", "Hỗ trợ ứng dụng Metro / UWP Store", 3, 34, 306),
    ("check_update", "Tự động kiểm tra phiên bản mới", 3, 34, 330),
];

thread_local! {
    static HWND_SELF: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    /// Set while the panel writes its own controls, so the resulting notifications are ignored
    static POPULATING: Cell<bool> = const { Cell::new(false) };
}

pub fn hwnd() -> HWND {
    HWND_SELF.with(|h| h.get())
}

/// Opens the panel (or brings it to the front), optionally on tab `tab`
pub fn show(tab: Option<usize>) {
    let mut hwnd = hwnd();
    if hwnd.is_null() {
        hwnd = controls::create_panel(CLASS, "Minkey - Bảng điều khiển", CLIENT);
        if hwnd.is_null() {
            return;
        }
        HWND_SELF.with(|h| h.set(hwnd));
        build(hwnd);
        refresh(hwnd);
        select_tab(hwnd, 0);
    }
    if let Some(tab) = tab {
        select_tab(hwnd, tab);
    }
    controls::bring_to_front(hwnd);
}

pub fn on_language_changed(lang: u32) {
    let hwnd = hwnd();
    if !hwnd.is_null() {
        populating(|| {
            controls::set_checked(controls::item(hwnd, ID_LANG_VI), lang != 0);
            controls::set_checked(controls::item(hwnd, ID_LANG_EN), lang == 0);
        });
    }
}

pub fn on_code_table_changed(table: u32) {
    let hwnd = hwnd();
    if !hwnd.is_null() {
        populating(|| controls::set_combo_index(controls::item(hwnd, ID_CODE_TABLE), table as i32));
    }
}

fn populating(f: impl FnOnce()) {
    POPULATING.with(|p| p.set(true));
    f();
    POPULATING.with(|p| p.set(false));
}

fn build(hwnd: HWND) {
    let mut f = Form::new(hwnd);

    f.label("Chế độ gõ:", (14, 15, 70, 20));
    f.radio(ID_LANG_VI, "Tiếng Việt", (90, 12, 100, 24), true);
    f.radio(ID_LANG_EN, "Tiếng Anh", (196, 12, 100, 24), false);

    let tab = f.add(WC_TABCONTROLW, "", WS_CLIPSIBLINGS | WS_TABSTOP | WS_GROUP, 0, ID_TAB, (10, 44, 540, 356), FontKind::Normal);
    for (i, name) in TABS.iter().enumerate() {
        let text = controls::wide(name);
        let item = TCITEMW {
            mask: TCIF_TEXT,
            dwState: 0,
            dwStateMask: 0,
            pszText: text.as_ptr() as *mut u16,
            cchTextMax: 0,
            iImage: -1,
            lParam: 0,
        };
        unsafe { SendMessageW(tab, TCM_INSERTITEMW, i, &item as *const _ as LPARAM) };
    }
    controls::update_page_background(tab);

    // ---- Page 1: Bộ gõ
    f.page = 1;
    f.group("Phương thức nhập liệu", (22, 76, 516, 60));
    f.label("Kiểu gõ:", (36, 103, 56, 20));
    f.combo(ID_INPUT_TYPE, &INPUT_TYPES, (96, 99, 160, 200));
    f.label("Bảng mã:", (276, 103, 60, 20));
    f.combo(ID_CODE_TABLE, &CODE_TABLES, (340, 99, 184, 200));

    f.group("Phím chuyển Anh / Việt", (22, 144, 516, 56));
    f.check(ID_SW_CTRL, "Ctrl", (36, 169, 52, 20));
    f.check(ID_SW_ALT, "Alt", (94, 169, 48, 20));
    f.check(ID_SW_WIN, "Win", (148, 169, 50, 20));
    f.check(ID_SW_SHIFT, "Shift", (204, 169, 56, 20));
    f.label("+", (266, 171, 12, 20));
    let key = f.edit(ID_SW_KEY, (282, 167, 36, 23), ES_UPPERCASE as u32 | ES_CENTER as u32);
    unsafe { SendMessageW(key, EM_SETLIMITTEXT, 1, 0) };
    f.check(ID_SW_BEEP, "Âm báo khi chuyển", (340, 169, 180, 20));

    f.group("Tùy chọn gõ", (22, 208, 516, 168));

    // ---- Page 2: Gõ tắt
    f.page = 2;
    f.group("Gõ tắt (Macro)", (22, 76, 516, 102));
    f.group("Gõ nhanh", (22, 188, 516, 102));
    f.button(ID_OPEN_MACRO, "Mở bảng gõ tắt…", (22, 304, 170, 30), false);
    f.button(ID_OPEN_CONVERT, "Công cụ chuyển mã…", (202, 304, 170, 30), false);

    // ---- Page 3: Hệ thống
    f.page = 3;
    f.group("Khởi động và giao diện", (22, 76, 516, 126));
    f.group("Nâng cao", (22, 212, 516, 150));

    // ---- Page 4: Thông tin
    f.page = 4;
    f.text("Minkey ⚡", (36, 70, 400, 44), FontKind::Title);
    f.text(&format!("Bộ gõ tiếng Việt mã nguồn mở · Phiên bản {}", env!("CARGO_PKG_VERSION")), (38, 118, 480, 22), FontKind::Bold);
    f.label(
        "Siêu nhẹ · Siêu nhanh · An toàn bộ nhớ tuyệt đối",
        (38, 144, 480, 20),
    );
    f.label(
        "Tác giả:      Hồ Hoàng Nam\nGiấy phép:  GNU General Public License v3.0\nNền tảng:    Windows 10 / 11  ·  Rust 2024 Edition",
        (38, 182, 480, 60),
    );
    f.button(ID_GITHUB, "GitHub", (38, 272, 110, 30), false);
    f.button(ID_CHECK_UPDATE, "Kiểm tra cập nhật", (158, 272, 150, 30), false);

    // Option check boxes, created after their group boxes so they are drawn on top
    for (i, (_, label, page, x, y)) in OPTIONS.iter().enumerate() {
        f.page = *page;
        // Two columns on the first tab, one full-width column elsewhere
        let width = if *page == 1 { 244 } else { 496 };
        f.check(ID_OPTION + i as i32, label, (*x, *y, width, 20));
    }

    // ---- Always visible
    f.page = 0;
    f.button(ID_RESET, "Cài đặt gốc", (10, 410, 110, 30), false);
    f.button(ID_HIDE, "Ẩn xuống khay", (330, 410, 110, 30), true);
    f.button(ID_EXIT, "Thoát Minkey", (450, 410, 100, 30), false);
}

/// Writes the current settings into every control
fn refresh(hwnd: HWND) {
    let ctx = ctx();
    let config = &ctx.config;
    let item = |id| controls::item(hwnd, id);
    populating(|| {
        let lang = config.language.load(Ordering::Relaxed);
        controls::set_checked(item(ID_LANG_VI), lang != 0);
        controls::set_checked(item(ID_LANG_EN), lang == 0);
        controls::set_combo_index(item(ID_INPUT_TYPE), config.input_type.load(Ordering::Relaxed) as i32);
        controls::set_combo_index(item(ID_CODE_TABLE), config.code_table.load(Ordering::Relaxed) as i32);

        let hk = settings::decode_hotkey(config.switch_key_status.load(Ordering::Relaxed));
        controls::set_checked(item(ID_SW_CTRL), hk.ctrl);
        controls::set_checked(item(ID_SW_ALT), hk.alt);
        controls::set_checked(item(ID_SW_WIN), hk.win);
        controls::set_checked(item(ID_SW_SHIFT), hk.shift);
        controls::set_text(item(ID_SW_KEY), &hk.key.map(String::from).unwrap_or_default());
        controls::set_checked(item(ID_SW_BEEP), hk.beep);

        for (i, (name, ..)) in OPTIONS.iter().enumerate() {
            controls::set_checked(item(ID_OPTION + i as i32), settings::get_setting(config, name));
        }
    });
}

fn select_tab(hwnd: HWND, tab: usize) {
    unsafe { SendMessageW(controls::item(hwnd, ID_TAB), TCM_SETCURSEL, tab, 0) };
    controls::show_page(hwnd, tab as u8 + 1);
}

fn read_switch_hotkey(hwnd: HWND) -> Hotkey {
    let item = |id| controls::item(hwnd, id);
    Hotkey {
        ctrl: controls::is_checked(item(ID_SW_CTRL)),
        alt: controls::is_checked(item(ID_SW_ALT)),
        win: controls::is_checked(item(ID_SW_WIN)),
        shift: controls::is_checked(item(ID_SW_SHIFT)),
        key: controls::get_text(item(ID_SW_KEY)).chars().next(),
        beep: controls::is_checked(item(ID_SW_BEEP)),
    }
}

fn on_command(hwnd: HWND, id: i32, code: u32) {
    if POPULATING.with(|p| p.get()) {
        return;
    }
    let ctx = ctx();
    match (id, code) {
        (ID_LANG_VI, BN_CLICKED) => settings::set_language(&ctx, 1),
        (ID_LANG_EN, BN_CLICKED) => settings::set_language(&ctx, 0),
        (ID_INPUT_TYPE, CBN_SELCHANGE) => {
            settings::set_input_type(&ctx, controls::combo_index(controls::item(hwnd, id)).max(0) as u32)
        }
        (ID_CODE_TABLE, CBN_SELCHANGE) => {
            settings::set_code_table(&ctx, controls::combo_index(controls::item(hwnd, id)).max(0) as u32)
        }
        (ID_SW_CTRL..=ID_SW_SHIFT | ID_SW_BEEP, BN_CLICKED) | (ID_SW_KEY, EN_CHANGE) => {
            settings::set_switch_hotkey(&ctx, &read_switch_hotkey(hwnd))
        }
        (ID_OPEN_MACRO, BN_CLICKED) => super::macro_window::show(),
        (ID_OPEN_CONVERT, BN_CLICKED) => super::convert_window::show(),
        (ID_GITHUB, BN_CLICKED) => crate::dialog::open_url("https://github.com/liamhnam/minkey"),
        (ID_CHECK_UPDATE, BN_CLICKED) => {
            controls::message_box(
                hwnd,
                "Minkey - Cập nhật",
                &format!("Bạn đang sử dụng phiên bản mới nhất! (Minkey v{})", env!("CARGO_PKG_VERSION")),
                MB_ICONINFORMATION | MB_OK,
            );
        }
        (ID_RESET, BN_CLICKED) => {
            let answer = controls::message_box(
                hwnd,
                "Minkey",
                "Bạn có chắc chắn muốn thiết lập lại cài đặt gốc?",
                MB_ICONQUESTION | MB_YESNO,
            );
            if answer == IDYES {
                settings::reset_defaults(&ctx);
                refresh(hwnd);
            }
        }
        (ID_HIDE, BN_CLICKED) | (IDCANCEL, _) | (IDOK, _) => unsafe {
            DestroyWindow(hwnd);
        },
        (ID_EXIT, BN_CLICKED) => super::exit(),
        _ if (ID_OPTION..ID_OPTION + OPTIONS.len() as i32).contains(&id) && code == BN_CLICKED => {
            let (name, ..) = OPTIONS[(id - ID_OPTION) as usize];
            let checked = controls::is_checked(controls::item(hwnd, id));
            if settings::apply_setting(&ctx, name, checked) == AfterSetting::Quit {
                super::exit();
            }
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
        WM_NOTIFY => {
            let hdr = unsafe { &*(lparam as *const NMHDR) };
            if hdr.idFrom == ID_TAB as usize && hdr.code == TCN_SELCHANGE {
                let sel = unsafe { SendMessageW(hdr.hwndFrom, TCM_GETCURSEL, 0, 0) };
                controls::show_page(hwnd, sel as u8 + 1);
            }
            0
        }
        WM_CTLCOLORSTATIC => controls::ctl_color(wparam as HDC, lparam as HWND),
        WM_DPICHANGED => {
            controls::on_dpi_changed(hwnd, wparam, lparam);
            0
        }
        WM_THEMECHANGED | WM_SYSCOLORCHANGE => {
            controls::update_page_background(controls::item(hwnd, ID_TAB));
            unsafe { InvalidateRect(hwnd, std::ptr::null(), 1) };
            unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) }
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
