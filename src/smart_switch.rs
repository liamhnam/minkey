// Smart Switch Key & Application Memory
// Ported 1:1 from OpenKey SmartSwitchKey.cpp & OpenKeyHelper.cpp

use std::collections::BTreeMap;
use std::sync::Mutex;
#[cfg(windows)]
use windows_sys::Win32::Foundation::*;
#[cfg(windows)]
use windows_sys::Win32::System::Threading::*;
#[cfg(windows)]
use windows_sys::Win32::UI::WindowsAndMessaging::*;

#[cfg(windows)]
#[link(name = "kernel32")]
unsafe extern "system" {
    pub fn QueryFullProcessImageNameW(
        h_process: HANDLE,
        dw_flags: u32,
        lp_exe_name: *mut u16,
        lpdw_size: *mut u32,
    ) -> i32;
}


static LAST_APP: Mutex<String> = Mutex::new(String::new());

#[derive(Debug, Clone, Default)]
pub struct SmartSwitchTable {
    pub map: BTreeMap<String, u8>,
    pub cache_key: String,
    pub cache_data: u8,
}

impl SmartSwitchTable {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
            cache_key: String::new(),
            cache_data: 0,
        }
    }

    pub fn from_binary(data: &[u8]) -> Self {
        let mut table = Self::new();
        if data.len() < 2 {
            return table;
        }

        let count = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut cursor = 2;

        for _ in 0..count {
            if cursor >= data.len() {
                break;
            }
            let name_len = data[cursor] as usize;
            cursor += 1;
            if cursor + name_len >= data.len() {
                break;
            }
            let app_name = String::from_utf8_lossy(&data[cursor..cursor + name_len]).to_string();
            cursor += name_len;
            if cursor >= data.len() {
                break;
            }
            let val = data[cursor];
            cursor += 1;
            table.map.insert(app_name, val);
        }

        table
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let count = self.map.len().min(u16::MAX as usize) as u16;
        bytes.extend_from_slice(&count.to_le_bytes());

        for (app_name, &val) in &self.map {
            let name_bytes = app_name.as_bytes();
            let name_len = name_bytes.len().min(255) as u8;
            bytes.push(name_len);
            bytes.extend_from_slice(&name_bytes[..name_len as usize]);
            bytes.push(val);
        }

        bytes
    }

    /// Queries the input method status for `app_name`.
    /// Returns the cached or stored status (>= 0).
    /// If not present, inserts `current_input_method` and returns -1 to indicate newly added (matches OpenKey C++).
    pub fn get_app_input_method_status(&mut self, app_name: &str, current_input_method: u8) -> i32 {
        if !self.cache_key.is_empty() && self.cache_key.eq_ignore_ascii_case(app_name) {
            return self.cache_data as i32;
        }
        for (k, &v) in &self.map {
            if k.eq_ignore_ascii_case(app_name) {
                self.cache_key = app_name.to_string();
                self.cache_data = v;
                return v as i32;
            }
        }
        self.cache_key = app_name.to_string();
        self.cache_data = current_input_method;
        self.map.insert(app_name.to_string(), current_input_method);
        -1
    }

    pub fn set_app_input_method_status(&mut self, app_name: &str, val: u8) {
        if app_name.is_empty() || app_name.eq_ignore_ascii_case("explorer.exe") {
            return;
        }
        self.map.insert(app_name.to_string(), val);
        self.cache_key = app_name.to_string();
        self.cache_data = val;
    }

    pub fn update(&mut self, app_name: &str, lang: u32, code_table: usize) {
        let val = ((lang & 0x01) as u8) | (((code_table.min(4) as u8) << 1) & 0xFE);
        self.set_app_input_method_status(app_name, val);
    }
}

pub fn get_last_app_name() -> Option<String> {
    if let Ok(last) = LAST_APP.lock() {
        if !last.is_empty() {
            return Some(last.clone());
        }
    }
    None
}

#[cfg(windows)]
pub fn get_frontmost_app_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return get_last_app_name();
        }

        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return get_last_app_name();
        }

        let proc = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if proc.is_null() {
            return get_last_app_name();
        }

        let mut buffer = [0u16; 1024];
        let mut size = 1024u32;
        let success = QueryFullProcessImageNameW(proc, 0, buffer.as_mut_ptr(), &mut size);
        CloseHandle(proc);

        if success != 0 && size > 0 {
            let full_path = String::from_utf16_lossy(&buffer[..size as usize]);
            if let Some(pos) = full_path.rfind('\\') {
                let exe = &full_path[pos + 1..];
                if !exe.eq_ignore_ascii_case("minkey.exe")
                    && !exe.eq_ignore_ascii_case("OpenKey64.exe")
                    && !exe.eq_ignore_ascii_case("OpenKey32.exe")
                    && !exe.eq_ignore_ascii_case("explorer.exe")
                {
                    if let Ok(mut last) = LAST_APP.lock() {
                        *last = exe.to_string();
                    }
                    return Some(exe.to_string());
                }
            }
        }
        get_last_app_name()
    }
}

/// Bundle identifier of the frontmost application (NSWorkspace), ignoring Minkey itself
#[cfg(target_os = "macos")]
pub fn get_frontmost_app_name() -> Option<String> {
    use objc2_app_kit::NSWorkspace;

    let id = objc2::rc::autoreleasepool(|_| {
        let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
        app.bundleIdentifier()
            .or_else(|| app.localizedName())
            .map(|s| s.to_string())
    });

    match id {
        Some(id)
            if !id.is_empty()
                && id != crate::MINKEY_BUNDLE_ID
                && !id.eq_ignore_ascii_case("minkey") =>
        {
            if let Ok(mut last) = LAST_APP.lock() {
                if *last != id {
                    *last = id.clone();
                }
            }
            Some(id)
        }
        _ => get_last_app_name(),
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn get_frontmost_app_name() -> Option<String> {
    get_last_app_name()
}

