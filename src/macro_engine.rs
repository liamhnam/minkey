// Macro Engine & Management
// Ported from OpenKey Macro.cpp with UniKey compatibility

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::tables::*;
use crate::types::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacroEntry {
    pub text: String,       // Shorthand abbreviation, e.g. "ko"
    pub content: String,    // Replacement text, e.g. "không"
}

#[derive(Debug, Clone, Default)]
pub struct MacroTable {
    pub map: BTreeMap<String, String>,
}

impl MacroTable {
    pub fn new() -> Self {
        Self {
            map: BTreeMap::new(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn add(&mut self, text: &str, content: &str) {
        if !text.trim().is_empty() && !content.is_empty() {
            self.map.insert(text.trim().to_string(), content.to_string());
        }
    }

    pub fn delete(&mut self, text: &str) -> bool {
        self.map.remove(text.trim()).is_some()
    }

    pub fn has(&self, text: &str) -> bool {
        self.map.contains_key(text.trim())
    }

    pub fn get_all(&self) -> Vec<MacroEntry> {
        self.map
            .iter()
            .map(|(k, v)| MacroEntry {
                text: k.clone(),
                content: v.clone(),
            })
            .collect()
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    // Binary format matching OpenKey registry 'macroData'
    // Byte 0-1: Total macro count (Uint16 LE)
    // For each macro:
    //   Byte: macroTextSize (Uint8)
    //   Bytes: macroText (UTF-8)
    //   Bytes: macroContentSize (Uint16 LE)
    //   Bytes: macroContent (UTF-8)
    pub fn from_binary(data: &[u8]) -> Self {
        let mut table = Self::new();
        if data.len() < 2 {
            return table;
        }

        let total = u16::from_le_bytes([data[0], data[1]]) as usize;
        let mut cursor = 2;

        for _ in 0..total {
            if cursor >= data.len() {
                break;
            }
            let text_len = data[cursor] as usize;
            cursor += 1;
            if cursor + text_len > data.len() {
                break;
            }
            let text = String::from_utf8_lossy(&data[cursor..cursor + text_len]).to_string();
            cursor += text_len;

            if cursor + 2 > data.len() {
                break;
            }
            let content_len = u16::from_le_bytes([data[cursor], data[cursor + 1]]) as usize;
            cursor += 2;
            if cursor + content_len > data.len() {
                break;
            }
            let content = String::from_utf8_lossy(&data[cursor..cursor + content_len]).to_string();
            cursor += content_len;

            table.add(&text, &content);
        }

        table
    }

    pub fn to_binary(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        let total = self.map.len().min(u16::MAX as usize) as u16;
        bytes.extend_from_slice(&total.to_le_bytes());

        for (text, content) in &self.map {
            let text_bytes = text.as_bytes();
            let content_bytes = content.as_bytes();
            let text_len = text_bytes.len().min(u8::MAX as usize) as u8;
            let content_len = content_bytes.len().min(u16::MAX as usize) as u16;

            bytes.push(text_len);
            bytes.extend_from_slice(&text_bytes[..text_len as usize]);
            bytes.extend_from_slice(&content_len.to_le_bytes());
            bytes.extend_from_slice(&content_bytes[..content_len as usize]);
        }

        bytes
    }

    // Import/Export in standard UniKey / OpenKey .txt file format
    pub fn import_txt(&mut self, text_content: &str, append: bool) {
        if !append {
            self.map.clear();
        }

        for line in text_content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with(';') {
                continue;
            }

            if let Some(pos) = trimmed.find(':') {
                let mut name = trimmed[..pos].trim();
                let mut content = &trimmed[pos + 1..];

                // Support colon inside shorthand name
                while name.is_empty() && !content.is_empty() {
                    if let Some(next_pos) = content.find(':') {
                        name = &content[..next_pos];
                        content = &content[next_pos + 1..];
                    } else {
                        break;
                    }
                }

                if !name.is_empty() {
                    self.add(name, content);
                }
            }
        }
    }

    pub fn export_txt(&self) -> String {
        let mut out = String::from(";Compatible OpenKey Macro Data file for UniKey*** version=1 ***\r\n");
        for (k, v) in &self.map {
            out.push_str(k);
            out.push(':');
            out.push_str(v);
            out.push_str("\r\n");
        }
        out
    }

    pub fn import_file<P: AsRef<Path>>(&mut self, path: P, append: bool) -> std::io::Result<usize> {
        let mut file = File::open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        let count_before = self.len();
        self.import_txt(&content, append);
        Ok(self.len() - count_before)
    }

    pub fn export_file<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        let content = self.export_txt();
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    // Match typed word and apply AutoCaps
    pub fn lookup(&self, word: &str, auto_caps: bool) -> Option<String> {
        if word.is_empty() {
            return None;
        }

        // Exact match first
        if let Some(replacement) = self.map.get(word) {
            return Some(replacement.clone());
        }

        if !auto_caps {
            return None;
        }

        // Check case-insensitive match
        let lower = word.to_lowercase();
        if let Some(base_replacement) = self.map.get(&lower) {
            let mut chars = word.chars();
            let first_is_upper = chars.next().map(|c| c.is_uppercase()).unwrap_or(false);
            let rest_is_upper = chars.all(|c| c.is_uppercase());

            if first_is_upper && rest_is_upper && word.chars().count() > 1 {
                // ALL CAPS, e.g. "KO" -> "KHÔNG"
                return Some(base_replacement.to_uppercase());
            } else if first_is_upper {
                // Title Case, e.g. "Ko" -> "Không"
                let mut rep_chars = base_replacement.chars();
                if let Some(first_ch) = rep_chars.next() {
                    let mut s = String::new();
                    for upper_c in first_ch.to_uppercase() {
                        s.push(upper_c);
                    }
                    s.push_str(rep_chars.as_str());
                    return Some(s);
                }
            }
        }

        None
    }
}

// Convert Unicode string into key code sequence for injection matching OpenKey convert()
pub fn string_to_macro_key_codes(s: &str, code_table: usize) -> Vec<u32> {
    let mut out = Vec::new();
    let tables = &*CODE_TABLES;
    let t0 = &tables[0];
    let target_table = &tables[code_table.min(4)];

    for ch in s.chars() {
        let code = ch as u32;

        // Standard character
        if let Some(&k) = CHARACTER_MAP.get(&code) {
            out.push(k);
            continue;
        }

        // Vietnamese vowel with diacritics
        let mut found = false;
        for (&root_key, v0) in t0.iter() {
            for (k, &char_code) in v0.iter().enumerate() {
                if char_code == (code as u16) {
                    if let Some(target_v) = target_table.get(&root_key) {
                        if k < target_v.len() {
                            out.push((target_v[k] as u32) | CHAR_CODE_MASK);
                            found = true;
                            break;
                        }
                    }
                }
            }
            if found {
                break;
            }
        }

        if !found {
            // Pure Unicode character
            out.push(code | PURE_CHARACTER_MASK);
        }
    }
    out
}

#[repr(C)]
#[allow(non_snake_case)]
pub struct OPENFILENAMEW {
    pub lStructSize: u32,
    pub hwndOwner: windows_sys::Win32::Foundation::HWND,
    pub hInstance: windows_sys::Win32::Foundation::HINSTANCE,
    pub lpstrFilter: *const u16,
    pub lpstrCustomFilter: *mut u16,
    pub nMaxCustFilter: u32,
    pub nFilterIndex: u32,
    pub lpstrFile: *mut u16,
    pub nMaxFile: u32,
    pub lpstrFileTitle: *mut u16,
    pub nMaxFileTitle: u32,
    pub lpstrInitialDir: *const u16,
    pub lpstrTitle: *const u16,
    pub Flags: u32,
    pub nFileOffset: u16,
    pub nFileExtension: u16,
    pub lpstrDefExt: *const u16,
    pub lCustData: isize,
    pub lpfnHook: Option<unsafe extern "system" fn(windows_sys::Win32::Foundation::HWND, u32, usize, isize) -> usize>,
    pub lpTemplateName: *const u16,
    pub pvReserved: *mut std::ffi::c_void,
    pub dwReserved: u32,
    pub FlagsEx: u32,
}

#[link(name = "comdlg32")]
unsafe extern "system" {
    pub fn GetOpenFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
    pub fn GetSaveFileNameW(lpofn: *mut OPENFILENAMEW) -> i32;
}

pub fn open_macro_file_dialog() -> Option<String> {
    unsafe {
        let mut buffer = [0u16; 1024];
        let filter: Vec<u16> = "Text file (*.txt)\0*.txt\0All (*.*)\0*.*\0\0".encode_utf16().collect();
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = 1024;
        ofn.Flags = 0x00001000 | 0x00000800;

        if GetOpenFileNameW(&mut ofn) != 0 {
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            Some(String::from_utf16_lossy(&buffer[..len]))
        } else {
            None
        }
    }
}

pub fn save_macro_file_dialog() -> Option<String> {
    unsafe {
        let mut buffer = [0u16; 1024];
        let default_name: Vec<u16> = "OpenKeyMacro.txt".encode_utf16().chain(std::iter::once(0)).collect();
        buffer[..default_name.len().min(1024)].copy_from_slice(&default_name[..default_name.len().min(1024)]);

        let filter: Vec<u16> = "Text file (*.txt)\0*.txt\0\0".encode_utf16().collect();
        let ext: Vec<u16> = "txt\0".encode_utf16().collect();
        let mut ofn: OPENFILENAMEW = std::mem::zeroed();
        ofn.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        ofn.lpstrFilter = filter.as_ptr();
        ofn.lpstrFile = buffer.as_mut_ptr();
        ofn.nMaxFile = 1024;
        ofn.lpstrDefExt = ext.as_ptr();
        ofn.Flags = 0x00000002 | 0x00000800;

        if GetSaveFileNameW(&mut ofn) != 0 {
            let len = buffer.iter().position(|&c| c == 0).unwrap_or(buffer.len());
            Some(String::from_utf16_lossy(&buffer[..len]))
        } else {
            None
        }
    }
}

pub fn ask_keep_existing_macros() -> bool {
    unsafe {
        let text: Vec<u16> = "Bạn có muốn giữ lại dữ liệu hiện tại không?\0".encode_utf16().collect();
        let title: Vec<u16> = "Dữ liệu gõ tắt\0".encode_utf16().collect();
        let res = windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW(
            std::ptr::null_mut(),
            text.as_ptr(),
            title.as_ptr(),
            windows_sys::Win32::UI::WindowsAndMessaging::MB_ICONEXCLAMATION | windows_sys::Win32::UI::WindowsAndMessaging::MB_YESNO,
        );
        res == windows_sys::Win32::UI::WindowsAndMessaging::IDYES
    }
}


