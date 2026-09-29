pub mod app;
pub mod config;
pub mod convert;
pub mod dialog;
pub mod engine;
pub mod hook;
#[cfg(target_os = "macos")]
pub mod hook_macos;
#[cfg(target_os = "macos")]
pub mod macos_app;
pub mod macro_engine;
pub mod smart_switch;
pub mod tables;
pub mod settings;
pub mod tray;
#[cfg(target_os = "macos")]
pub mod tray_macos;
pub mod types;
#[cfg(windows)]
pub mod win32_ui;

/// CFBundleIdentifier of the macOS app bundle (see bundle_macos.sh)
pub const MINKEY_BUNDLE_ID: &str = "org.minkey.Minkey";

pub use config::*;
pub use convert::*;
pub use engine::VietnameseEngine;
pub use macro_engine::*;
pub use smart_switch::*;
pub use types::*;

/// Releases unused working set memory back to the operating system
pub fn trim_process_memory() {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::System::ProcessStatus::EmptyWorkingSet;
        use windows_sys::Win32::System::Threading::GetCurrentProcess;
        EmptyWorkingSet(GetCurrentProcess());
    }
    #[cfg(target_os = "macos")]
    unsafe {
        unsafe extern "C" {
            fn malloc_zone_pressure_relief(zone: *mut std::ffi::c_void, goal: usize) -> usize;
        }
        malloc_zone_pressure_relief(std::ptr::null_mut(), 0);
    }
}




