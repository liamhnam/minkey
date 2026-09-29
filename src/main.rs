// Minkey - Vietnamese input method for Windows (Rust port of OpenKey)

#![windows_subsystem = "windows"]

#[cfg(not(windows))]
compile_error!("Minkey hiện chỉ build cho Windows (giao diện Win32).");

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateMutexW(attributes: *const std::ffi::c_void, initial_owner: i32, name: *const u16) -> HANDLE;
}

fn wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Only one Minkey per session: a second launch opens the running one's control panel and exits
fn check_single_instance() -> bool {
    unsafe {
        let mutex_name = wide_null("Local\\MinkeyAppMutex");
        SetLastError(0);
        let _mutex = CreateMutexW(std::ptr::null(), 1, mutex_name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            let tray_wnd = FindWindowW(wide_null("MinkeyTrayWndClass").as_ptr(), std::ptr::null());
            if !tray_wnd.is_null() {
                PostMessageW(tray_wnd, minkey::tray::WM_TRAYMESSAGE, 0, WM_LBUTTONDBLCLK as isize);
            }
            minkey::hook::MessageBeep(0);
            return false;
        }
    }
    true
}

fn main() {
    if check_single_instance() {
        minkey::win32_ui::run();
    }
}
