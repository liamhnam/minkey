// Small helpers over raw Win32 controls.
//
// Every control remembers its layout in its own GWLP_USERDATA (rectangle in 96-DPI units, tab page
// and font), so a window can be re-laid out for a new DPI or switch tab pages by enumerating its
// children, without keeping a separate list of controls.

use std::cell::RefCell;
use std::collections::HashMap;

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::HiDpi::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::Win32::UI::Controls::{BST_CHECKED, BST_UNCHECKED, CloseThemeData, DrawThemeBackground, OpenThemeData, TABP_PANE};
use windows_sys::core::{BOOL, PCWSTR, w};

// Static control styles (defined in System::SystemServices, not worth another feature)
const SS_LEFT: u32 = 0x0;
const SS_NOPREFIX: u32 = 0x80;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 96-DPI length -> pixels at `dpi`
pub fn scale(v: i32, dpi: u32) -> i32 {
    (v * dpi as i32 + 48) / 96
}

/// (x, y, width, height) in 96-DPI units
pub type Rect = (i32, i32, i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FontKind {
    Normal = 0,
    Bold = 1,
    Title = 2,
}

impl FontKind {
    fn from_bits(v: isize) -> Self {
        match v {
            1 => FontKind::Bold,
            2 => FontKind::Title,
            _ => FontKind::Normal,
        }
    }
}

thread_local! {
    // A handful of fonts per DPI, kept for the life of the UI thread
    static FONTS: RefCell<HashMap<(u32, FontKind), HFONT>> = RefCell::new(HashMap::new());
}

pub fn font(dpi: u32, kind: FontKind) -> HFONT {
    FONTS.with(|fonts| {
        *fonts.borrow_mut().entry((dpi, kind)).or_insert_with(|| {
            let (points, weight) = match kind {
                FontKind::Normal => (9, FW_NORMAL),
                FontKind::Bold => (9, FW_SEMIBOLD),
                FontKind::Title => (20, FW_BOLD),
            };
            let face = wide("Segoe UI");
            unsafe {
                CreateFontW(
                    -((points * dpi as i32 + 36) / 72),
                    0, 0, 0,
                    weight as i32,
                    0, 0, 0,
                    DEFAULT_CHARSET as u32,
                    OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32,
                    CLEARTYPE_QUALITY as u32,
                    DEFAULT_PITCH as u32,
                    face.as_ptr(),
                )
            }
        })
    })
}

// ---------------------------------------------------------------- layout tag

const TAG_MARK: isize = 1 << 62;

#[derive(Debug, Clone, Copy)]
struct Tag {
    rect: Rect,
    page: u8,
    font: FontKind,
}

fn pack(rect: Rect, page: u8, font: FontKind) -> isize {
    let f = |v: i32| (v as isize) & 0xFFF;
    TAG_MARK
        | f(rect.0)
        | (f(rect.1) << 12)
        | (f(rect.2) << 24)
        | (f(rect.3) << 36)
        | (((page as isize) & 0x3F) << 48)
        | ((font as isize) << 54)
}

fn tag_of(ctl: HWND) -> Option<Tag> {
    let v = unsafe { GetWindowLongPtrW(ctl, GWLP_USERDATA) };
    if v & TAG_MARK == 0 {
        return None; // not one of ours (e.g. the list view's header)
    }
    let g = |shift: u32| ((v >> shift) & 0xFFF) as i32;
    Some(Tag {
        rect: (g(0), g(12), g(24), g(36)),
        page: ((v >> 48) & 0x3F) as u8,
        font: FontKind::from_bits((v >> 54) & 0x3),
    })
}

fn for_each_child(parent: HWND, mut f: impl FnMut(HWND, Tag)) {
    unsafe extern "system" fn visit(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let f = unsafe { &mut *(lparam as *mut &mut dyn FnMut(HWND, Tag)) };
        if let Some(tag) = tag_of(hwnd) {
            f(hwnd, tag);
        }
        1
    }
    let mut f: &mut dyn FnMut(HWND, Tag) = &mut f;
    unsafe { EnumChildWindows(parent, Some(visit), &mut f as *mut _ as LPARAM) };
}

/// Moves and re-fonts every control for `dpi`
pub fn relayout(parent: HWND, dpi: u32) {
    for_each_child(parent, |ctl, tag| unsafe {
        let (x, y, w, h) = tag.rect;
        SetWindowPos(ctl, std::ptr::null_mut(), scale(x, dpi), scale(y, dpi), scale(w, dpi), scale(h, dpi),
            SWP_NOZORDER | SWP_NOACTIVATE);
        SendMessageW(ctl, WM_SETFONT, font(dpi, tag.font) as WPARAM, 1);
    });
}

/// Shows the controls of tab `page` (1-based) and hides the other pages; page 0 is always visible
pub fn show_page(parent: HWND, page: u8) {
    for_each_child(parent, |ctl, tag| unsafe {
        if tag.page != 0 {
            ShowWindow(ctl, if tag.page == page { SW_SHOW } else { SW_HIDE });
        }
    });
}

thread_local! {
    /// Brush and colour of the tab body, see `update_page_background`
    static PAGE_BACKGROUND: std::cell::Cell<(HBRUSH, COLORREF)> =
        const { std::cell::Cell::new((std::ptr::null_mut(), 0)) };
}

/// Reads the colour the theme paints the tab body with (light grey on Windows 11, not white),
/// so labels and check boxes on a tab page blend in. Call again on WM_THEMECHANGED.
pub fn update_page_background(tab: HWND) {
    let color = unsafe {
        let theme = OpenThemeData(tab, w!("TAB"));
        if theme == 0 {
            GetSysColor(COLOR_BTNFACE) // classic / high-contrast: tabs use the dialog colour
        } else {
            let screen = GetDC(std::ptr::null_mut());
            let dc = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, 32, 32);
            let old = SelectObject(dc, bitmap);
            let rc = RECT { left: 0, top: 0, right: 32, bottom: 32 };
            DrawThemeBackground(theme, dc, TABP_PANE, 0, &rc, std::ptr::null());
            let color = GetPixel(dc, 16, 16);
            SelectObject(dc, old);
            DeleteObject(bitmap);
            DeleteDC(dc);
            ReleaseDC(std::ptr::null_mut(), screen);
            CloseThemeData(theme);
            color
        }
    };
    PAGE_BACKGROUND.with(|bg| {
        let (old, _) = bg.get();
        if !old.is_null() {
            unsafe { DeleteObject(old) };
        }
        bg.set((unsafe { CreateSolidBrush(color) }, color));
    });
}

/// WM_CTLCOLORSTATIC: labels / check boxes sit on the tab body or on the dialog-grey frame
pub fn ctl_color(hdc: HDC, ctl: HWND) -> LRESULT {
    let on_page = tag_of(ctl).is_some_and(|t| t.page != 0);
    let (page_brush, page_color) = PAGE_BACKGROUND.with(|bg| bg.get());
    unsafe {
        SetTextColor(hdc, GetSysColor(COLOR_WINDOWTEXT));
        if on_page && !page_brush.is_null() {
            SetBkColor(hdc, page_color);
            page_brush as LRESULT
        } else {
            SetBkColor(hdc, GetSysColor(COLOR_BTNFACE));
            GetSysColorBrush(COLOR_BTNFACE) as LRESULT
        }
    }
}

// ---------------------------------------------------------------- building

/// Adds controls to `hwnd`; `page` is the tab page they belong to (0 = outside the tabs)
pub struct Form {
    pub hwnd: HWND,
    pub dpi: u32,
    pub page: u8,
}

/// Id for controls that never send commands
pub const NO_ID: i32 = 0xFFFF;

impl Form {
    pub fn new(hwnd: HWND) -> Self {
        Self { hwnd, dpi: unsafe { GetDpiForWindow(hwnd) }, page: 0 }
    }

    pub fn add(&self, class: PCWSTR, text: &str, style: u32, ex_style: u32, id: i32, rect: Rect, kind: FontKind) -> HWND {
        let (x, y, w, h) = rect;
        let visible = if self.page == 0 { WS_VISIBLE } else { 0 };
        let text = wide(text);
        unsafe {
            let ctl = CreateWindowExW(
                ex_style,
                class,
                text.as_ptr(),
                WS_CHILD | visible | style,
                scale(x, self.dpi), scale(y, self.dpi), scale(w, self.dpi), scale(h, self.dpi),
                self.hwnd,
                id as usize as HMENU,
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            SendMessageW(ctl, WM_SETFONT, font(self.dpi, kind) as WPARAM, 0);
            SetWindowLongPtrW(ctl, GWLP_USERDATA, pack(rect, self.page, kind));
            ctl
        }
    }

    pub fn label(&self, text: &str, rect: Rect) -> HWND {
        self.add(w!("STATIC"), text, SS_LEFT | SS_NOPREFIX, 0, NO_ID, rect, FontKind::Normal)
    }

    pub fn text(&self, text: &str, rect: Rect, kind: FontKind) -> HWND {
        self.add(w!("STATIC"), text, SS_LEFT | SS_NOPREFIX, 0, NO_ID, rect, kind)
    }

    pub fn group(&self, text: &str, rect: Rect) -> HWND {
        self.add(w!("BUTTON"), text, BS_GROUPBOX as u32, 0, NO_ID, rect, FontKind::Bold)
    }

    pub fn check(&self, id: i32, text: &str, rect: Rect) -> HWND {
        self.add(w!("BUTTON"), text, BS_AUTOCHECKBOX as u32 | WS_TABSTOP, 0, id, rect, FontKind::Normal)
    }

    /// `first` starts a new group of mutually exclusive radio buttons
    pub fn radio(&self, id: i32, text: &str, rect: Rect, first: bool) -> HWND {
        let group = if first { WS_GROUP | WS_TABSTOP } else { 0 };
        self.add(w!("BUTTON"), text, BS_AUTORADIOBUTTON as u32 | group, 0, id, rect, FontKind::Normal)
    }

    pub fn button(&self, id: i32, text: &str, rect: Rect, default: bool) -> HWND {
        let kind = if default { BS_DEFPUSHBUTTON } else { BS_PUSHBUTTON };
        self.add(w!("BUTTON"), text, kind as u32 | WS_TABSTOP, 0, id, rect, FontKind::Normal)
    }

    /// Drop-down list; `rect.3` is the height of the opened list
    pub fn combo(&self, id: i32, items: &[&str], rect: Rect) -> HWND {
        let ctl = self.add(w!("COMBOBOX"), "", CBS_DROPDOWNLIST as u32 | WS_VSCROLL | WS_TABSTOP, 0, id, rect, FontKind::Normal);
        for item in items {
            let text = wide(item);
            unsafe { SendMessageW(ctl, CB_ADDSTRING, 0, text.as_ptr() as LPARAM) };
        }
        ctl
    }

    pub fn edit(&self, id: i32, rect: Rect, style: u32) -> HWND {
        self.add(w!("EDIT"), "", ES_AUTOHSCROLL as u32 | WS_TABSTOP | style, WS_EX_CLIENTEDGE, id, rect, FontKind::Normal)
    }
}

// ---------------------------------------------------------------- reading / writing controls

pub fn item(parent: HWND, id: i32) -> HWND {
    unsafe { GetDlgItem(parent, id) }
}

pub fn get_text(ctl: HWND) -> String {
    unsafe {
        let len = GetWindowTextLengthW(ctl);
        let mut buf = vec![0u16; len as usize + 1];
        let n = GetWindowTextW(ctl, buf.as_mut_ptr(), buf.len() as i32);
        String::from_utf16_lossy(&buf[..n.max(0) as usize])
    }
}

pub fn set_text(ctl: HWND, text: &str) {
    let text = wide(text);
    unsafe { SetWindowTextW(ctl, text.as_ptr()) };
}

pub fn is_checked(ctl: HWND) -> bool {
    unsafe { SendMessageW(ctl, BM_GETCHECK, 0, 0) == BST_CHECKED as LRESULT }
}

pub fn set_checked(ctl: HWND, checked: bool) {
    let state = if checked { BST_CHECKED } else { BST_UNCHECKED };
    unsafe { SendMessageW(ctl, BM_SETCHECK, state as WPARAM, 0) };
}

pub fn combo_index(ctl: HWND) -> i32 {
    unsafe { SendMessageW(ctl, CB_GETCURSEL, 0, 0) as i32 }
}

pub fn set_combo_index(ctl: HWND, index: i32) {
    unsafe { SendMessageW(ctl, CB_SETCURSEL, index.max(0) as WPARAM, 0) };
}

pub fn message_box(owner: HWND, title: &str, text: &str, flags: MESSAGEBOX_STYLE) -> MESSAGEBOX_RESULT {
    let (title, text) = (wide(title), wide(text));
    unsafe { MessageBoxW(owner, text.as_ptr(), title.as_ptr(), flags) }
}

// ---------------------------------------------------------------- top-level windows

pub const PANEL_STYLE: WINDOW_STYLE = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX;

/// Creates a hidden panel window whose client area is `client` (96-DPI units), centred on its monitor
pub fn create_panel(class: PCWSTR, title: &str, client: (i32, i32)) -> HWND {
    let title = wide(title);
    unsafe {
        let hwnd = CreateWindowExW(
            0, class, title.as_ptr(), PANEL_STYLE,
            CW_USEDEFAULT, CW_USEDEFAULT, 100, 100,
            std::ptr::null_mut(), std::ptr::null_mut(), GetModuleHandleW(std::ptr::null()), std::ptr::null(),
        );
        if !hwnd.is_null() {
            let dpi = GetDpiForWindow(hwnd);
            let (w, h) = frame_size(client, dpi);
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi);
            let work = mi.rcWork;
            let x = work.left + ((work.right - work.left) - w) / 2;
            let y = work.top + ((work.bottom - work.top) - h) / 2;
            SetWindowPos(hwnd, std::ptr::null_mut(), x, y, w, h, SWP_NOZORDER | SWP_NOACTIVATE);
        }
        hwnd
    }
}

/// Outer window size for a client area of `client` (96-DPI units)
pub fn frame_size(client: (i32, i32), dpi: u32) -> (i32, i32) {
    let mut rc = RECT { left: 0, top: 0, right: scale(client.0, dpi), bottom: scale(client.1, dpi) };
    unsafe { AdjustWindowRectExForDpi(&mut rc, PANEL_STYLE, 0, 0, dpi) };
    (rc.right - rc.left, rc.bottom - rc.top)
}

/// WM_DPICHANGED: take the size Windows suggests for the new monitor and scale the controls
pub fn on_dpi_changed(hwnd: HWND, wparam: WPARAM, lparam: LPARAM) {
    let dpi = (wparam & 0xFFFF) as u32;
    let rc = unsafe { &*(lparam as *const RECT) };
    unsafe {
        SetWindowPos(hwnd, std::ptr::null_mut(), rc.left, rc.top, rc.right - rc.left, rc.bottom - rc.top,
            SWP_NOZORDER | SWP_NOACTIVATE);
    }
    relayout(hwnd, dpi);
}

/// Brings an existing panel to the front
pub fn bring_to_front(hwnd: HWND) {
    unsafe {
        if IsIconic(hwnd) != 0 {
            ShowWindow(hwnd, SW_RESTORE);
        }
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
    }
}
