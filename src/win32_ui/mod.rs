// Native Win32 control panel.
//
// The UI thread owns a hidden message-only window: the tray and keyboard-hook threads post
// `UiCommand`s to it. Panels are created when opened and destroyed when closed, so while Minkey
// sits in the tray it keeps no window, font or control alive beyond this hidden window.

mod controls;
mod convert_window;
mod macro_window;
mod main_window;

use std::cell::{Cell, OnceCell};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Graphics::Gdi::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;
use windows_sys::core::{PCWSTR, w};

use crate::app::{AppContext, Services, UiCommand};

const APP_CLASS: PCWSTR = w!("MinkeyAppWnd");
const WM_APP_COMMAND: u32 = WM_APP + 1;
const WM_APP_TRIM: u32 = WM_APP + 2;

thread_local! {
    static CTX: OnceCell<AppContext> = const { OnceCell::new() };
    static APP_HWND: Cell<HWND> = const { Cell::new(std::ptr::null_mut()) };
    static ICON: Cell<HICON> = const { Cell::new(std::ptr::null_mut()) };
    /// Panels only, no tray: quit once the last panel is closed
    static PREVIEW: Cell<bool> = const { Cell::new(false) };
}

fn ctx() -> AppContext {
    CTX.with(|c| c.get().cloned()).expect("UI used before the app context was set")
}

/// Starts Minkey (tray + keyboard hook) and runs the UI until Exit
pub fn run() {
    init_classes();
    let app_hwnd = create_app_window();
    let target = app_hwnd as usize;
    let (ctx, services) = crate::app::start(move |cmd| post_command(target, cmd));
    run_with(ctx, Some(services));
}

/// Runs the UI with an existing context. `services` (tray + hook) is stopped on exit.
/// Without services every panel opens at once, which is how `examples/panels.rs` previews them.
pub fn run_with(ctx: AppContext, services: Option<Services>) {
    init_classes();
    if APP_HWND.with(|h| h.get()).is_null() {
        create_app_window();
    }
    let show_on_startup = ctx.config.show_on_startup.load(std::sync::atomic::Ordering::Relaxed);
    let config = ctx.config.clone();
    CTX.with(|c| {
        let _ = c.set(ctx);
    });

    if services.is_none() {
        // Preview: open every panel
        PREVIEW.with(|p| p.set(true));
        main_window::show(None);
        macro_window::show();
        convert_window::show();
    } else {
        crate::hook::MessageBeep(0);
        if show_on_startup {
            main_window::show(None);
        } else {
            crate::trim_process_memory();
        }
    }

    message_loop();

    if let Some(services) = services {
        services.stop();
    }
    config.save_to_registry();
}

fn message_loop() {
    unsafe {
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            // Tab / Enter / Esc keyboard navigation inside the panels
            let root = GetAncestor(msg.hwnd, GA_ROOT);
            if !root.is_null() && is_panel(root) && IsDialogMessageW(root, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}

fn is_panel(hwnd: HWND) -> bool {
    hwnd == main_window::hwnd() || hwnd == macro_window::hwnd() || hwnd == convert_window::hwnd()
}

// ---------------------------------------------------------------- commands from other threads

fn post_command(app_hwnd: usize, cmd: UiCommand) {
    let (code, arg) = match cmd {
        UiCommand::ShowControlPanel => (1, 0),
        UiCommand::ShowAbout => (2, 0),
        UiCommand::ShowMacroTable => (3, 0),
        UiCommand::ShowConvertTool => (4, 0),
        UiCommand::LanguageChanged(lang) => (5, lang as isize),
        UiCommand::CodeTableChanged(table) => (6, table as isize),
        UiCommand::Exit => (7, 0),
    };
    unsafe { PostMessageW(app_hwnd as HWND, WM_APP_COMMAND, code, arg) };
}

fn handle_command(code: usize, arg: isize) {
    match code {
        1 => main_window::show(None),
        2 => main_window::show(Some(main_window::ABOUT_TAB)),
        3 => macro_window::show(),
        4 => convert_window::show(),
        5 => main_window::on_language_changed(arg as u32),
        6 => main_window::on_code_table_changed(arg as u32),
        7 => exit(),
        _ => {}
    }
}

/// Closes the panels and ends the message loop
pub(crate) fn exit() {
    for hwnd in [main_window::hwnd(), macro_window::hwnd(), convert_window::hwnd()] {
        if !hwnd.is_null() {
            unsafe { DestroyWindow(hwnd) };
        }
    }
    unsafe { PostQuitMessage(0) };
}

/// Called when a panel is destroyed: give memory back once no panel is left
fn panel_closed() {
    let app = APP_HWND.with(|h| h.get());
    unsafe { PostMessageW(app, WM_APP_TRIM, 0, 0) };
}

unsafe extern "system" fn app_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_APP_COMMAND => {
            handle_command(wparam, lparam);
            0
        }
        WM_APP_TRIM => {
            if main_window::hwnd().is_null() && macro_window::hwnd().is_null() && convert_window::hwnd().is_null() {
                if PREVIEW.with(|p| p.get()) {
                    unsafe { PostQuitMessage(0) };
                } else {
                    crate::trim_process_memory();
                }
            }
            0
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

// ---------------------------------------------------------------- window classes

fn init_classes() {
    thread_local!(static DONE: Cell<bool> = const { Cell::new(false) });
    if DONE.with(|d| d.replace(true)) {
        return;
    }
    unsafe {
        let icc = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_LISTVIEW_CLASSES | ICC_TAB_CLASSES,
        };
        InitCommonControlsEx(&icc);

        let icon = crate::tray::create_tray_icon(true, false);
        ICON.with(|i| i.set(icon));

        register_class(APP_CLASS, app_wnd_proc);
        register_class(main_window::CLASS, main_window::wnd_proc);
        register_class(macro_window::CLASS, macro_window::wnd_proc);
        register_class(convert_window::CLASS, convert_window::wnd_proc);
    }
}

unsafe fn register_class(name: PCWSTR, proc: unsafe extern "system" fn(HWND, u32, WPARAM, LPARAM) -> LRESULT) {
    let icon = ICON.with(|i| i.get());
    unsafe {
        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(std::ptr::null()),
            hIcon: icon,
            hCursor: LoadCursorW(std::ptr::null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_BTNFACE + 1) as usize as HBRUSH,
            lpszMenuName: std::ptr::null(),
            lpszClassName: name,
            hIconSm: icon,
        };
        RegisterClassExW(&wc);
    }
}

fn create_app_window() -> HWND {
    let hwnd = unsafe {
        CreateWindowExW(
            0, APP_CLASS, w!("MinkeyApp"), 0, 0, 0, 0, 0,
            HWND_MESSAGE, std::ptr::null_mut(), GetModuleHandleW(std::ptr::null()), std::ptr::null(),
        )
    };
    APP_HWND.with(|h| h.set(hwnd));
    hwnd
}
