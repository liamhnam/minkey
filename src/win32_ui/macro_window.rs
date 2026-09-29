// Macro table: list, search, add / edit / delete, import / export (.txt)

use std::cell::{Cell, RefCell};
use std::sync::atomic::Ordering;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::HDC;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{SetFocus, VK_DELETE};
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{PCWSTR, w};

use super::controls::{self, FontKind, Form};
use super::ctx;
use crate::macro_engine::MacroEntry;
use crate::settings;

pub const CLASS: PCWSTR = w!("MinkeyMacroWnd");
const CLIENT: (i32, i32) = (600, 470);

const ID_SEARCH: i32 = 301;
const ID_LIST: i32 = 302;
const ID_TEXT: i32 = 303;
const ID_CONTENT: i32 = 304;
const ID_AUTO_CAPS: i32 = 305;
const ID_SAVE: i32 = 306;
const ID_DELETE: i32 = 307;
const ID_IMPORT: i32 = 308;
const ID_EXPORT: i32 = 309;
const ID_CLOSE: i32 = 310;

/// Column widths in 96-DPI units
const COLUMNS: [(&str, i32); 2] = [("Từ gõ tắt", 150), ("Nội dung thay thế", 400)];

thread_local! {
    static HWND_SELF: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    static POPULATING: Cell<bool> = const { Cell::new(false) };
    /// Rows currently shown in the list, in list order
    static ROWS: RefCell<Vec<MacroEntry>> = const { RefCell::new(Vec::new()) };
}

pub fn hwnd() -> HWND {
    HWND_SELF.with(|h| h.get())
}

pub fn show() {
    let mut hwnd = hwnd();
    if hwnd.is_null() {
        hwnd = controls::create_panel(CLASS, "Minkey - Bảng gõ tắt", CLIENT);
        if hwnd.is_null() {
            return;
        }
        HWND_SELF.with(|h| h.set(hwnd));
        build(hwnd);
    }
    populating(|| {
        let auto_caps = ctx().config.auto_caps_macro.load(Ordering::Relaxed);
        controls::set_checked(controls::item(hwnd, ID_AUTO_CAPS), auto_caps);
    });
    fill_list(hwnd, None);
    controls::bring_to_front(hwnd);
}

fn populating(f: impl FnOnce()) {
    POPULATING.with(|p| p.set(true));
    f();
    POPULATING.with(|p| p.set(false));
}

fn build(hwnd: HWND) {
    let f = Form::new(hwnd);
    f.label("Tìm kiếm:", (12, 15, 70, 20));
    f.edit(ID_SEARCH, (84, 11, 504, 24), 0);

    let list = f.add(
        WC_LISTVIEWW,
        "",
        LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS | WS_TABSTOP | WS_BORDER,
        0,
        ID_LIST,
        (12, 44, 576, 258),
        FontKind::Normal,
    );
    unsafe {
        let ex = LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER | LVS_EX_GRIDLINES;
        SendMessageW(list, LVM_SETEXTENDEDLISTVIEWSTYLE, ex as WPARAM, ex as LPARAM);
        for (i, (title, width)) in COLUMNS.iter().enumerate() {
            let text = controls::wide(title);
            let mut col: LVCOLUMNW = std::mem::zeroed();
            col.mask = LVCF_TEXT | LVCF_WIDTH;
            col.cx = controls::scale(*width, f.dpi);
            col.pszText = text.as_ptr() as *mut u16;
            SendMessageW(list, LVM_INSERTCOLUMNW, i, &col as *const _ as LPARAM);
        }
    }

    f.label("Từ gõ tắt:", (12, 314, 150, 20));
    f.edit(ID_TEXT, (12, 334, 150, 24), 0);
    f.label("Nội dung thay thế:", (172, 314, 200, 20));
    f.edit(ID_CONTENT, (172, 334, 416, 24), 0);
    f.check(ID_AUTO_CAPS, "Tự đổi hoa/thường theo từ viết tắt (ko → không, Ko → Không, KO → KHÔNG)", (12, 366, 576, 20));

    f.button(ID_SAVE, "Thêm", (12, 394, 110, 30), true);
    f.button(ID_DELETE, "Xoá", (130, 394, 90, 30), false);
    f.button(ID_IMPORT, "Nạp từ file (.txt)…", (12, 430, 150, 30), false);
    f.button(ID_EXPORT, "Xuất ra file (.txt)…", (170, 430, 150, 30), false);
    f.button(ID_CLOSE, "Đóng", (488, 430, 100, 30), false);
}

/// Reloads the rows matching the search box; selects `select` if given
fn fill_list(hwnd: HWND, select: Option<&str>) {
    let ctx = ctx();
    let filter = controls::get_text(controls::item(hwnd, ID_SEARCH));
    let rows = settings::macro_entries(&ctx, &filter);
    let list = controls::item(hwnd, ID_LIST);
    let mut selected = None;
    unsafe {
        SendMessageW(list, WM_SETREDRAW, 0, 0);
        SendMessageW(list, LVM_DELETEALLITEMS, 0, 0);
        for (i, row) in rows.iter().enumerate() {
            let text = controls::wide(&row.text);
            let mut item: LVITEMW = std::mem::zeroed();
            item.mask = LVIF_TEXT;
            item.iItem = i as i32;
            item.pszText = text.as_ptr() as *mut u16;
            SendMessageW(list, LVM_INSERTITEMW, 0, &item as *const _ as LPARAM);

            let content = controls::wide(&row.content);
            item.iSubItem = 1;
            item.pszText = content.as_ptr() as *mut u16;
            SendMessageW(list, LVM_SETITEMTEXTW, i, &item as *const _ as LPARAM);

            if select == Some(row.text.as_str()) {
                selected = Some(i);
            }
        }
        SendMessageW(list, WM_SETREDRAW, 1, 0);
    }
    ROWS.with(|r| *r.borrow_mut() = rows);
    if let Some(i) = selected {
        populating(|| unsafe {
            let mut state: LVITEMW = std::mem::zeroed();
            state.stateMask = LVIS_SELECTED | LVIS_FOCUSED;
            state.state = LVIS_SELECTED | LVIS_FOCUSED;
            SendMessageW(list, LVM_SETITEMSTATE, i, &state as *const _ as LPARAM);
            SendMessageW(list, LVM_ENSUREVISIBLE, i, 0);
        });
    }
}

fn selected_row(hwnd: HWND) -> Option<MacroEntry> {
    let list = controls::item(hwnd, ID_LIST);
    let index = unsafe { SendMessageW(list, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as LPARAM) };
    if index < 0 {
        return None;
    }
    ROWS.with(|r| r.borrow().get(index as usize).cloned())
}

/// Shows "Cập nhật" when the shorthand already exists, "Thêm" otherwise
fn update_save_label(hwnd: HWND) {
    let text = controls::get_text(controls::item(hwnd, ID_TEXT));
    let exists = settings::macro_content(&ctx(), &text).is_some();
    controls::set_text(controls::item(hwnd, ID_SAVE), if exists { "Cập nhật" } else { "Thêm" });
}

fn save(hwnd: HWND) {
    let text = controls::get_text(controls::item(hwnd, ID_TEXT));
    let content = controls::get_text(controls::item(hwnd, ID_CONTENT));
    if !settings::save_macro(&ctx(), &text, &content) {
        controls::message_box(hwnd, "Minkey - Gõ tắt", "Hãy nhập cả từ gõ tắt và nội dung thay thế.", MB_ICONWARNING | MB_OK);
        return;
    }
    fill_list(hwnd, Some(text.trim()));
    update_save_label(hwnd);
}

fn delete(hwnd: HWND) {
    let typed = controls::get_text(controls::item(hwnd, ID_TEXT));
    let text = if typed.trim().is_empty() { selected_row(hwnd).map(|r| r.text) } else { Some(typed) };
    let Some(text) = text else { return };
    if settings::delete_macro(&ctx(), &text) {
        populating(|| {
            controls::set_text(controls::item(hwnd, ID_TEXT), "");
            controls::set_text(controls::item(hwnd, ID_CONTENT), "");
        });
        fill_list(hwnd, None);
        update_save_label(hwnd);
    }
}

fn import(hwnd: HWND) {
    let Some(path) = crate::macro_engine::open_macro_file_dialog() else { return };
    let keep = controls::message_box(
        hwnd,
        "Dữ liệu gõ tắt",
        "Bạn có muốn giữ lại dữ liệu hiện tại không?",
        MB_ICONQUESTION | MB_YESNO,
    ) == IDYES;
    match settings::import_macros(&ctx(), &path, keep) {
        Ok(_) => fill_list(hwnd, None),
        Err(e) => {
            controls::message_box(hwnd, "Minkey - Gõ tắt", &format!("Không đọc được file:\n{e}"), MB_ICONERROR | MB_OK);
        }
    }
}

fn export(hwnd: HWND) {
    let Some(path) = crate::macro_engine::save_macro_file_dialog() else { return };
    if let Err(e) = settings::export_macros(&ctx(), &path) {
        controls::message_box(hwnd, "Minkey - Gõ tắt", &format!("Không ghi được file:\n{e}"), MB_ICONERROR | MB_OK);
    }
}

fn on_command(hwnd: HWND, id: i32, code: u32) {
    if POPULATING.with(|p| p.get()) {
        return;
    }
    match (id, code) {
        (ID_SEARCH, EN_CHANGE) => fill_list(hwnd, None),
        (ID_TEXT, EN_CHANGE) => {
            // Typing an existing shorthand loads its text for editing
            let text = controls::get_text(controls::item(hwnd, ID_TEXT));
            if let Some(content) = settings::macro_content(&ctx(), &text) {
                populating(|| controls::set_text(controls::item(hwnd, ID_CONTENT), &content));
            }
            update_save_label(hwnd);
        }
        (ID_AUTO_CAPS, BN_CLICKED) => {
            let checked = controls::is_checked(controls::item(hwnd, id));
            settings::apply_setting(&ctx(), "macro_auto_caps", checked);
        }
        (ID_SAVE, BN_CLICKED) | (IDOK, _) => save(hwnd),
        (ID_DELETE, BN_CLICKED) => delete(hwnd),
        (ID_IMPORT, BN_CLICKED) => import(hwnd),
        (ID_EXPORT, BN_CLICKED) => export(hwnd),
        (ID_CLOSE, BN_CLICKED) | (IDCANCEL, _) => unsafe {
            DestroyWindow(hwnd);
        },
        _ => {}
    }
}

fn on_notify(hwnd: HWND, hdr: &NMHDR, lparam: LPARAM) {
    if hdr.idFrom != ID_LIST as usize || POPULATING.with(|p| p.get()) {
        return;
    }
    match hdr.code {
        LVN_ITEMCHANGED => {
            let change = unsafe { &*(lparam as *const NMLISTVIEW) };
            if change.uNewState & LVIS_SELECTED != 0 && change.uOldState & LVIS_SELECTED == 0 {
                if let Some(row) = ROWS.with(|r| r.borrow().get(change.iItem as usize).cloned()) {
                    populating(|| {
                        controls::set_text(controls::item(hwnd, ID_TEXT), &row.text);
                        controls::set_text(controls::item(hwnd, ID_CONTENT), &row.content);
                    });
                    update_save_label(hwnd);
                }
            }
        }
        LVN_KEYDOWN => {
            let key = unsafe { &*(lparam as *const NMLVKEYDOWN) };
            if key.wVKey == VK_DELETE {
                if let Some(row) = selected_row(hwnd) {
                    populating(|| controls::set_text(controls::item(hwnd, ID_TEXT), &row.text));
                    delete(hwnd);
                }
            }
        }
        NM_DBLCLK => unsafe {
            SetFocus(controls::item(hwnd, ID_CONTENT));
        },
        _ => {}
    }
}

fn rescale_columns(hwnd: HWND) {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    let list = controls::item(hwnd, ID_LIST);
    for (i, (_, width)) in COLUMNS.iter().enumerate() {
        unsafe { SendMessageW(list, LVM_SETCOLUMNWIDTH, i, controls::scale(*width, dpi) as LPARAM) };
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
            on_notify(hwnd, hdr, lparam);
            0
        }
        WM_CTLCOLORSTATIC => controls::ctl_color(wparam as HDC, lparam as HWND),
        WM_DPICHANGED => {
            controls::on_dpi_changed(hwnd, wparam, lparam);
            rescale_columns(hwnd);
            0
        }
        WM_CLOSE => {
            unsafe { DestroyWindow(hwnd) };
            0
        }
        WM_DESTROY => {
            HWND_SELF.with(|h| h.set(std::ptr::null_mut()));
            ROWS.with(|r| r.borrow_mut().clear());
            super::panel_closed();
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}
