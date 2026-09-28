// Configuration management matching OpenKey registry schema: HKCU\Software\TuyenMai\OpenKey

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::System::Registry::*;
use windows_sys::Win32::System::LibraryLoader::GetModuleFileNameW;

#[link(name = "advapi32")]
unsafe extern "system" {
    pub fn RegCreateKeyExW(
        hkey: HKEY,
        lpsubkey: *const u16,
        reserved: u32,
        lpclass: *const u16,
        dwoptions: u32,
        samdesired: u32,
        lpsecurityattributes: *const std::ffi::c_void,
        phkresult: *mut HKEY,
        lpdwdisposition: *mut u32,
    ) -> i32;
}


pub const REGISTRY_KEY_PATH: &str = "Software\\TuyenMai\\OpenKey";
pub const RUN_ON_STARTUP_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";

pub const DEFAULT_SWITCH_STATUS: u32 = 0x7A000206; // Alt + Z
pub const EMPTY_HOTKEY: u32 = 0x00FE;

fn to_wide_null(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub language: Arc<AtomicU32>,                  // 0: English, 1: Vietnamese
    pub input_type: Arc<AtomicU32>,                // 0: Telex, 1: VNI, 2: Simple Telex 1, 3: Simple Telex 2
    pub code_table: Arc<AtomicU32>,                // 0: Unicode, 1: TCVN3, 2: VNI Win, 3: Compound, 4: 1258
    pub check_spelling: Arc<AtomicBool>,           // Kiểm tra chính tả
    pub use_modern_orthography: Arc<AtomicBool>,   // Đặt dấu kiểu mới (òa -> oà)
    pub quick_telex: Arc<AtomicBool>,              // Gõ nhanh telex
    pub switch_key_status: Arc<AtomicU32>,         // Phím chuyển E/V
    pub restore_if_wrong_spelling: Arc<AtomicBool>,// Tự phục hồi từ sai chính tả
    pub fix_recommend_browser: Arc<AtomicBool>,    // Sửa lỗi gợi ý trình duyệt
    pub use_macro: Arc<AtomicBool>,                // Bật gõ tắt
    pub use_macro_in_english: Arc<AtomicBool>,     // Bật gõ tắt ở chế độ tiếng Anh
    pub auto_caps_macro: Arc<AtomicBool>,          // Viết hoa theo từ viết tắt
    pub send_key_step_by_step: Arc<AtomicBool>,    // Gửi phím từng bước
    pub use_gray_icon: Arc<AtomicBool>,            // Dùng icon xám trên taskbar
    pub show_on_startup: Arc<AtomicBool>,          // Hiện bảng điều khiển khi khởi động
    pub run_with_windows: Arc<AtomicBool>,         // Khởi động cùng Windows
    pub use_smart_switch_key: Arc<AtomicBool>,     // Tự chuyển E/V theo ứng dụng
    pub upper_case_first_char: Arc<AtomicBool>,    // Tự viết hoa đầu câu
    pub allow_consonant_zfwj: Arc<AtomicBool>,     // Cho phép phụ âm z, f, w, j
    pub temp_off_spelling: Arc<AtomicBool>,        // Tạm tắt kiểm tra chính tả (nhấn đúp Ctrl)
    pub quick_start_consonant: Arc<AtomicBool>,    // Phụ âm đầu thông minh (f->ph, j->gi, w->qu)
    pub quick_end_consonant: Arc<AtomicBool>,      // Phụ âm cuối thông minh (g->ng, h->nh, k->ch)
    pub support_metro_app: Arc<AtomicBool>,        // Hỗ trợ ứng dụng Metro/UWP
    pub run_as_admin: Arc<AtomicBool>,             // Chạy với quyền quản trị
    pub create_desktop_shortcut: Arc<AtomicBool>,  // Tạo shortcut Desktop
    pub check_new_version: Arc<AtomicBool>,        // Kiểm tra cập nhật
    pub remember_code: Arc<AtomicBool>,            // Ghi nhớ bảng mã theo ứng dụng
    pub other_language: Arc<AtomicBool>,           // Hỗ trợ gõ song ngữ
    pub temp_off_openkey: Arc<AtomicBool>,         // Tạm tắt bộ gõ
    pub fix_chromium_browser: Arc<AtomicBool>,     // Sửa lỗi Chromium

    // Convert tool settings
    pub convert_dont_alert: Arc<AtomicBool>,
    pub convert_to_all_caps: Arc<AtomicBool>,
    pub convert_to_all_non_caps: Arc<AtomicBool>,
    pub convert_to_caps_first_letter: Arc<AtomicBool>,
    pub convert_to_caps_each_word: Arc<AtomicBool>,
    pub convert_remove_mark: Arc<AtomicBool>,
    pub convert_from_code: Arc<AtomicU32>,
    pub convert_to_code: Arc<AtomicU32>,
    pub convert_hotkey: Arc<AtomicU32>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: Arc::new(AtomicU32::new(1)),
            input_type: Arc::new(AtomicU32::new(0)),
            code_table: Arc::new(AtomicU32::new(0)),
            check_spelling: Arc::new(AtomicBool::new(true)),
            use_modern_orthography: Arc::new(AtomicBool::new(false)),
            quick_telex: Arc::new(AtomicBool::new(false)),
            switch_key_status: Arc::new(AtomicU32::new(DEFAULT_SWITCH_STATUS)),
            restore_if_wrong_spelling: Arc::new(AtomicBool::new(true)),
            fix_recommend_browser: Arc::new(AtomicBool::new(true)),
            use_macro: Arc::new(AtomicBool::new(true)),
            use_macro_in_english: Arc::new(AtomicBool::new(false)),
            auto_caps_macro: Arc::new(AtomicBool::new(false)),
            send_key_step_by_step: Arc::new(AtomicBool::new(true)),
            use_gray_icon: Arc::new(AtomicBool::new(false)),
            show_on_startup: Arc::new(AtomicBool::new(false)),
            run_with_windows: Arc::new(AtomicBool::new(true)),
            use_smart_switch_key: Arc::new(AtomicBool::new(true)),
            upper_case_first_char: Arc::new(AtomicBool::new(false)),
            allow_consonant_zfwj: Arc::new(AtomicBool::new(false)),
            temp_off_spelling: Arc::new(AtomicBool::new(false)),
            quick_start_consonant: Arc::new(AtomicBool::new(false)),
            quick_end_consonant: Arc::new(AtomicBool::new(false)),
            support_metro_app: Arc::new(AtomicBool::new(false)),
            run_as_admin: Arc::new(AtomicBool::new(false)),
            create_desktop_shortcut: Arc::new(AtomicBool::new(false)),
            check_new_version: Arc::new(AtomicBool::new(false)),
            remember_code: Arc::new(AtomicBool::new(true)),
            other_language: Arc::new(AtomicBool::new(true)),
            temp_off_openkey: Arc::new(AtomicBool::new(false)),
            fix_chromium_browser: Arc::new(AtomicBool::new(false)),

            convert_dont_alert: Arc::new(AtomicBool::new(false)),
            convert_to_all_caps: Arc::new(AtomicBool::new(false)),
            convert_to_all_non_caps: Arc::new(AtomicBool::new(false)),
            convert_to_caps_first_letter: Arc::new(AtomicBool::new(false)),
            convert_to_caps_each_word: Arc::new(AtomicBool::new(false)),
            convert_remove_mark: Arc::new(AtomicBool::new(false)),
            convert_from_code: Arc::new(AtomicU32::new(0)),
            convert_to_code: Arc::new(AtomicU32::new(0)),
            convert_hotkey: Arc::new(AtomicU32::new(EMPTY_HOTKEY)),
        }
    }
}

impl AppConfig {
    pub fn to_hook_config(&self) -> crate::hook::HookConfig {
        crate::hook::HookConfig {
            language: self.language.clone(),
            switch_key_status: self.switch_key_status.clone(),
            fix_recommend_browser: self.fix_recommend_browser.clone(),
            fix_chromium_browser: self.fix_chromium_browser.clone(),
            support_metro_app: self.support_metro_app.clone(),
            send_key_step_by_step: self.send_key_step_by_step.clone(),
            temp_off_spelling: self.temp_off_spelling.clone(),
            temp_off_openkey: self.temp_off_openkey.clone(),
            use_smart_switch_key: self.use_smart_switch_key.clone(),
            remember_code: self.remember_code.clone(),
            convert_tool_hotkey: self.convert_hotkey.clone(),
            use_macro: self.use_macro.clone(),
            use_macro_in_english: self.use_macro_in_english.clone(),
        }
    }
}



pub fn get_reg_int(key_name: &str, default_value: u32) -> u32 {
    unsafe {
        let sub_key = to_wide_null(REGISTRY_KEY_PATH);
        let mut h_key: HKEY = std::ptr::null_mut();
        let ret = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            KEY_READ,
            &mut h_key,
        );
        if ret != ERROR_SUCCESS {
            return default_value;
        }

        let val_name = to_wide_null(key_name);
        let mut data: u32 = 0;
        let mut data_size: u32 = std::mem::size_of::<u32>() as u32;
        let query_ret = RegQueryValueExW(
            h_key,
            val_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );

        RegCloseKey(h_key);

        if query_ret == ERROR_SUCCESS {
            data
        } else {
            default_value
        }
    }
}

pub fn set_reg_int(key_name: &str, value: u32) {
    unsafe {
        let sub_key = to_wide_null(REGISTRY_KEY_PATH);
        let mut h_key: HKEY = std::ptr::null_mut();
        let ret = RegCreateKeyExW(
            HKEY_CURRENT_USER,
            sub_key.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_WRITE,
            std::ptr::null(),
            &mut h_key,
            std::ptr::null_mut(),
        );
        if ret != ERROR_SUCCESS as i32 {
            return;
        }


        let val_name = to_wide_null(key_name);
        let _ = RegSetValueExW(
            h_key,
            val_name.as_ptr(),
            0,
            REG_DWORD,
            &value as *const u32 as *const u8,
            std::mem::size_of::<u32>() as u32,
        );

        RegCloseKey(h_key);
    }
}

pub fn get_reg_binary(key_name: &str) -> Option<Vec<u8>> {
    unsafe {
        let sub_key = to_wide_null(REGISTRY_KEY_PATH);
        let mut h_key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_READ, &mut h_key) != ERROR_SUCCESS {
            return None;
        }

        let val_name = to_wide_null(key_name);
        let mut data_size: u32 = 0;
        if RegQueryValueExW(h_key, val_name.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), &mut data_size) != ERROR_SUCCESS || data_size == 0 {
            RegCloseKey(h_key);
            return None;
        }

        let mut buffer = vec![0u8; data_size as usize];
        let query_ret = RegQueryValueExW(
            h_key,
            val_name.as_ptr(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            buffer.as_mut_ptr(),
            &mut data_size,
        );

        RegCloseKey(h_key);

        if query_ret == ERROR_SUCCESS {
            Some(buffer)
        } else {
            None
        }
    }
}

pub fn set_reg_binary(key_name: &str, data: &[u8]) {
    unsafe {
        let sub_key = to_wide_null(REGISTRY_KEY_PATH);
        let mut h_key: HKEY = std::ptr::null_mut();
        if RegCreateKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, std::ptr::null(), REG_OPTION_NON_VOLATILE, KEY_WRITE, std::ptr::null(), &mut h_key, std::ptr::null_mut()) != ERROR_SUCCESS as i32 {
            return;
        }


        let val_name = to_wide_null(key_name);
        let _ = RegSetValueExW(
            h_key,
            val_name.as_ptr(),
            0,
            REG_BINARY,
            data.as_ptr(),
            data.len() as u32,
        );

        RegCloseKey(h_key);
    }
}

impl AppConfig {
    pub fn load_from_registry() -> Self {
        let config = Self::default();

        config.language.store(get_reg_int("vLanguage", 1), Ordering::Relaxed);
        config.input_type.store(get_reg_int("vInputType", 0), Ordering::Relaxed);
        config.code_table.store(get_reg_int("vCodeTable", 0), Ordering::Relaxed);
        config.check_spelling.store(get_reg_int("vCheckSpelling", 1) != 0, Ordering::Relaxed);
        config.use_modern_orthography.store(get_reg_int("vUseModernOrthography", 0) != 0, Ordering::Relaxed);
        config.quick_telex.store(get_reg_int("vQuickTelex", 0) != 0, Ordering::Relaxed);
        config.switch_key_status.store(get_reg_int("vSwitchKeyStatus", DEFAULT_SWITCH_STATUS), Ordering::Relaxed);
        config.restore_if_wrong_spelling.store(get_reg_int("vRestoreIfWrongSpelling", 1) != 0, Ordering::Relaxed);
        config.fix_recommend_browser.store(get_reg_int("vFixRecommendBrowser", 1) != 0, Ordering::Relaxed);
        config.use_macro.store(get_reg_int("vUseMacro", 1) != 0, Ordering::Relaxed);
        config.use_macro_in_english.store(get_reg_int("vUseMacroInEnglishMode", 0) != 0, Ordering::Relaxed);
        config.auto_caps_macro.store(get_reg_int("vAutoCapsMacro", 0) != 0, Ordering::Relaxed);
        config.send_key_step_by_step.store(get_reg_int("vSendKeyStepByStep", 1) != 0, Ordering::Relaxed);
        config.use_gray_icon.store(get_reg_int("vUseGrayIcon", 0) != 0, Ordering::Relaxed);
        config.show_on_startup.store(get_reg_int("vShowOnStartUp", 1) != 0, Ordering::Relaxed);
        config.run_with_windows.store(get_reg_int("vRunWithWindows", 1) != 0, Ordering::Relaxed);
        config.use_smart_switch_key.store(get_reg_int("vUseSmartSwitchKey", 1) != 0, Ordering::Relaxed);
        config.upper_case_first_char.store(get_reg_int("vUpperCaseFirstChar", 0) != 0, Ordering::Relaxed);
        config.allow_consonant_zfwj.store(get_reg_int("vAllowConsonantZFWJ", 0) != 0, Ordering::Relaxed);
        config.temp_off_spelling.store(get_reg_int("vTempOffSpelling", 0) != 0, Ordering::Relaxed);
        config.quick_start_consonant.store(get_reg_int("vQuickStartConsonant", 0) != 0, Ordering::Relaxed);
        config.quick_end_consonant.store(get_reg_int("vQuickEndConsonant", 0) != 0, Ordering::Relaxed);
        config.support_metro_app.store(get_reg_int("vSupportMetroApp", 0) != 0, Ordering::Relaxed);
        config.run_as_admin.store(get_reg_int("vRunAsAdmin", 0) != 0, Ordering::Relaxed);
        config.create_desktop_shortcut.store(get_reg_int("vCreateDesktopShortcut", 0) != 0, Ordering::Relaxed);
        config.check_new_version.store(get_reg_int("vCheckNewVersion", 0) != 0, Ordering::Relaxed);
        config.remember_code.store(get_reg_int("vRememberCode", 1) != 0, Ordering::Relaxed);
        config.other_language.store(get_reg_int("vOtherLanguage", 1) != 0, Ordering::Relaxed);
        config.temp_off_openkey.store(get_reg_int("vTempOffOpenKey", 0) != 0, Ordering::Relaxed);
        config.fix_chromium_browser.store(get_reg_int("vFixChromiumBrowser", 0) != 0, Ordering::Relaxed);

        config.convert_dont_alert.store(get_reg_int("convertToolDontAlertWhenCompleted", 0) != 0, Ordering::Relaxed);
        config.convert_to_all_caps.store(get_reg_int("convertToolToAllCaps", 0) != 0, Ordering::Relaxed);
        config.convert_to_all_non_caps.store(get_reg_int("convertToolToAllNonCaps", 0) != 0, Ordering::Relaxed);
        config.convert_to_caps_first_letter.store(get_reg_int("convertToolToCapsFirstLetter", 0) != 0, Ordering::Relaxed);
        config.convert_to_caps_each_word.store(get_reg_int("convertToolToCapsEachWord", 0) != 0, Ordering::Relaxed);
        config.convert_remove_mark.store(get_reg_int("convertToolRemoveMark", 0) != 0, Ordering::Relaxed);
        config.convert_from_code.store(get_reg_int("convertToolFromCode", 0), Ordering::Relaxed);
        config.convert_to_code.store(get_reg_int("convertToolToCode", 0), Ordering::Relaxed);
        config.convert_hotkey.store(get_reg_int("convertToolHotKey", EMPTY_HOTKEY), Ordering::Relaxed);

        config
    }

    pub fn save_to_registry(&self) {
        set_reg_int("vLanguage", self.language.load(Ordering::Relaxed));
        set_reg_int("vInputType", self.input_type.load(Ordering::Relaxed));
        set_reg_int("vCodeTable", self.code_table.load(Ordering::Relaxed));
        set_reg_int("vCheckSpelling", if self.check_spelling.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUseModernOrthography", if self.use_modern_orthography.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vQuickTelex", if self.quick_telex.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vSwitchKeyStatus", self.switch_key_status.load(Ordering::Relaxed));
        set_reg_int("vRestoreIfWrongSpelling", if self.restore_if_wrong_spelling.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vFixRecommendBrowser", if self.fix_recommend_browser.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUseMacro", if self.use_macro.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUseMacroInEnglishMode", if self.use_macro_in_english.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vAutoCapsMacro", if self.auto_caps_macro.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vSendKeyStepByStep", if self.send_key_step_by_step.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUseGrayIcon", if self.use_gray_icon.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vShowOnStartUp", if self.show_on_startup.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vRunWithWindows", if self.run_with_windows.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUseSmartSwitchKey", if self.use_smart_switch_key.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vUpperCaseFirstChar", if self.upper_case_first_char.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vAllowConsonantZFWJ", if self.allow_consonant_zfwj.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vTempOffSpelling", if self.temp_off_spelling.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vQuickStartConsonant", if self.quick_start_consonant.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vQuickEndConsonant", if self.quick_end_consonant.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vSupportMetroApp", if self.support_metro_app.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vRunAsAdmin", if self.run_as_admin.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vCreateDesktopShortcut", if self.create_desktop_shortcut.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vCheckNewVersion", if self.check_new_version.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vRememberCode", if self.remember_code.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vOtherLanguage", if self.other_language.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vTempOffOpenKey", if self.temp_off_openkey.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("vFixChromiumBrowser", if self.fix_chromium_browser.load(Ordering::Relaxed) { 1 } else { 0 });

        set_reg_int("convertToolDontAlertWhenCompleted", if self.convert_dont_alert.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolToAllCaps", if self.convert_to_all_caps.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolToAllNonCaps", if self.convert_to_all_non_caps.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolToCapsFirstLetter", if self.convert_to_caps_first_letter.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolToCapsEachWord", if self.convert_to_caps_each_word.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolRemoveMark", if self.convert_remove_mark.load(Ordering::Relaxed) { 1 } else { 0 });
        set_reg_int("convertToolFromCode", self.convert_from_code.load(Ordering::Relaxed));
        set_reg_int("convertToolToCode", self.convert_to_code.load(Ordering::Relaxed));
        set_reg_int("convertToolHotKey", self.convert_hotkey.load(Ordering::Relaxed));
    }
}

pub fn get_executable_path() -> String {
    unsafe {
        let mut buffer = [0u16; 1024];
        let len = GetModuleFileNameW(std::ptr::null_mut(), buffer.as_mut_ptr(), 1024);
        if len > 0 {
            String::from_utf16_lossy(&buffer[..len as usize])
        } else {
            String::new()
        }
    }
}

pub fn register_run_on_startup(enable: bool, run_as_admin: bool) {
    let exe_path = get_executable_path();
    if exe_path.is_empty() {
        return;
    }

    if enable {
        if run_as_admin {
            let cmd = format!("schtasks /create /sc onlogon /tn Minkey /rl highest /tr \"{}\" /f", exe_path);
            let _ = std::process::Command::new("cmd")
                .args(["/C", &cmd])
                .status();
        } else {
            unsafe {
                let sub_key = to_wide_null(RUN_ON_STARTUP_KEY);
                let mut h_key: HKEY = std::ptr::null_mut();
                if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_WRITE, &mut h_key) == ERROR_SUCCESS {
                    let val_name = to_wide_null("OpenKey");
                    let path_wide = to_wide_null(&exe_path);
                    let _ = RegSetValueExW(
                        h_key,
                        val_name.as_ptr(),
                        0,
                        REG_SZ,
                        path_wide.as_ptr() as *const u8,
                        (path_wide.len() * std::mem::size_of::<u16>()) as u32,
                    );
                    RegCloseKey(h_key);
                }
            }
        }
    } else {
        unsafe {
            let sub_key = to_wide_null(RUN_ON_STARTUP_KEY);
            let mut h_key: HKEY = std::ptr::null_mut();
            if RegOpenKeyExW(HKEY_CURRENT_USER, sub_key.as_ptr(), 0, KEY_WRITE, &mut h_key) == ERROR_SUCCESS {
                let val_name = to_wide_null("OpenKey");
                let _ = RegDeleteValueW(h_key, val_name.as_ptr());
                RegCloseKey(h_key);
            }
        }
        let _ = std::process::Command::new("schtasks")
            .args(["/delete", "/tn", "Minkey", "/f"])
            .status();
    }
}

#[link(name = "shell32")]
unsafe extern "system" {
    pub fn IsUserAnAdmin() -> i32;
    pub fn ShellExecuteW(
        hwnd: HWND,
        lpoperation: *const u16,
        lpfile: *const u16,
        lpparameters: *const u16,
        lpdirectory: *const u16,
        nshowcmd: i32,
    ) -> isize;
}

pub fn is_user_an_admin() -> bool {
    unsafe { IsUserAnAdmin() != 0 }
}

pub fn relaunch_as_admin() {
    let exe = get_executable_path();
    if exe.is_empty() {
        return;
    }
    unsafe {
        let op = to_wide_null("runas");
        let path = to_wide_null(&exe);
        ShellExecuteW(
            std::ptr::null_mut(),
            op.as_ptr(),
            path.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}

pub fn create_desktop_shortcut() {
    let exe = get_executable_path();
    if exe.is_empty() {
        return;
    }
    use std::os::windows::process::CommandExt;
    let clean_exe = exe.replace('\'', "''");
    let script = format!(
        "$ws = New-Object -ComObject WScript.Shell; $d = [Environment]::GetFolderPath('Desktop'); $s = $ws.CreateShortcut(\"$d\\Minkey.lnk\"); $s.TargetPath = '{}'; $s.Description = 'Minkey - Bo go Tieng Viet'; $s.Save()",
        clean_exe
    );
    let _ = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .status();
}

