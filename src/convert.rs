// Encoding & Text Conversion Tool
// Ported 1:1 from OpenKey ConvertTool.cpp & OpenKeyHelper.cpp

#[cfg(windows)]
use std::sync::LazyLock;
#[cfg(windows)]
use windows_sys::Win32::System::DataExchange::*;
#[cfg(windows)]
use windows_sys::Win32::System::Memory::*;

use crate::tables::*;

#[cfg(windows)]
pub const CF_UNICODETEXT: u32 = 13;

#[cfg(windows)]
pub static CF_HTML: LazyLock<u32> = LazyLock::new(|| unsafe {
    let name: Vec<u16> = "HTML Format\0".encode_utf16().collect();
    RegisterClipboardFormatW(name.as_ptr())
});

#[cfg(windows)]
pub static CF_RTF: LazyLock<u32> = LazyLock::new(|| unsafe {
    let name: Vec<u16> = "Rich Text Format\0".encode_utf16().collect();
    RegisterClipboardFormatW(name.as_ptr())
});


#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertOptions {
    pub from_code: usize,            // 0: Unicode, 1: TCVN3, 2: VNI, 3: Compound, 4: 1258
    pub to_code: usize,              // 0: Unicode, 1: TCVN3, 2: VNI, 3: Compound, 4: 1258
    pub to_all_caps: bool,           // Chuyển sang chữ HOA
    pub to_all_non_caps: bool,       // Chuyển sang chữ thường
    pub to_caps_first_letter: bool,  // Viết hoa chữ cái đầu câu
    pub to_caps_each_word: bool,     // Viết hoa mỗi từ
    pub remove_mark: bool,           // Loại bỏ dấu tiếng Việt
    pub dont_alert: bool,            // Không hiện thông báo khi xong
}

impl Default for ConvertOptions {
    fn default() -> Self {
        Self {
            from_code: 0,
            to_code: 0,
            to_all_caps: false,
            to_all_non_caps: false,
            to_caps_first_letter: false,
            to_caps_each_word: false,
            remove_mark: false,
            dont_alert: false,
        }
    }
}

fn get_unicode_compound_mark_index(mark: u16) -> u16 {
    for (i, &m) in UNICODE_COMPOUND_MARK.iter().enumerate() {
        if mark == m {
            return ((i + 1) as u16) << 13;
        }
    }
    0
}

fn find_key_code(char_code: u32, code_table_idx: usize, should_upper_case: bool) -> Option<(u32, usize)> {
    let tables = &*CODE_TABLES;
    if code_table_idx >= tables.len() {
        return None;
    }
    let table = &tables[code_table_idx];
    for (&root_key, entries) in table.iter() {
        for (z, &entry_code) in entries.iter().enumerate() {
            if char_code == entry_code as u32 {
                // If TCVN3 has identical codes for upper and lower in this vowel slot
                if code_table_idx == 1 {
                    if (z % 2 == 0) && (z + 1 < entries.len()) && entries[z] == entries[z + 1] {
                        if !should_upper_case {
                            return Some((root_key, z + 1));
                        }
                    }
                }
                return Some((root_key, z));
            }
        }
    }
    None
}

// Convert string between any pair of Vietnamese code tables with casing/diacritic transformations
pub fn convert_util(source: &str, opts: &ConvertOptions) -> String {
    let data: Vec<u16> = source.encode_utf16().collect();
    let mut temp: Vec<u16> = Vec::with_capacity(data.len() + 16);

    let has_casing_transform = opts.to_all_caps
        || opts.to_all_non_caps
        || opts.to_caps_first_letter
        || opts.to_caps_each_word;

    let mut should_upper_case = opts.to_caps_first_letter || opts.to_caps_each_word;
    if opts.to_all_non_caps {
        should_upper_case = false;
    }

    let mut has_break = false;
    let from_table_idx = opts.from_code.min(4);
    let to_table_idx = opts.to_code.min(4);
    let tables = &*CODE_TABLES;
    let to_table = &tables[to_table_idx];

    let mut i = 0;
    while i < data.len() {
        // Check two-code-unit character for VNI (2), CP 1258 (4), and Unicode Compound (3)
        if i + 1 < data.len() {
            let (t, p) = match from_table_idx {
                2 | 4 => ((data[i] as u32) | ((data[i + 1] as u32) << 8), 1),
                3 => {
                    let target_mark = get_unicode_compound_mark_index(data[i + 1]);
                    if target_mark > 0 {
                        ((data[i] as u32) | (target_mark as u32), 1)
                    } else {
                        (data[i] as u32, 0)
                    }
                }
                _ => (data[i] as u32, 0),
            };

            if let Some((j, k)) = find_key_code(t, from_table_idx, should_upper_case) {
                i += p;
                emit_target(
                    j,
                    k,
                    opts,
                    has_casing_transform,
                    should_upper_case,
                    to_table_idx,
                    to_table,
                    &mut temp,
                );
                should_upper_case = false;
                has_break = false;
                i += 1;
                continue;
            }
        }

        // Single code unit lookup
        let t = data[i] as u32;
        if let Some((j, k)) = find_key_code(t, from_table_idx, should_upper_case) {
            emit_target(
                j,
                k,
                opts,
                has_casing_transform,
                should_upper_case,
                to_table_idx,
                to_table,
                &mut temp,
            );
            should_upper_case = false;
            has_break = false;
            i += 1;
            continue;
        }

        // Standard / non-Vietnamese characters
        let ch = match char::from_u32(data[i] as u32) {
            Some(c) => c,
            None => {
                temp.push(data[i]);
                i += 1;
                continue;
            }
        };

        if opts.to_all_caps || (has_casing_transform && should_upper_case) {
            for up in ch.to_uppercase() {
                temp.push(up as u16);
            }
        } else if opts.to_all_non_caps || (has_casing_transform && !should_upper_case) {
            for low in ch.to_lowercase() {
                temp.push(low as u16);
            }
        } else {
            temp.push(data[i]);
        }

        if t == ('\n' as u32) || (has_break && t == (' ' as u32)) {
            if opts.to_caps_first_letter || opts.to_caps_each_word {
                should_upper_case = true;
            }
        } else if t == (' ' as u32) && opts.to_caps_each_word {
            should_upper_case = true;
        } else if t == ('.' as u32) || t == ('?' as u32) || t == ('!' as u32) {
            has_break = true;
        } else {
            should_upper_case = false;
            has_break = false;
        }

        i += 1;
    }

    String::from_utf16_lossy(&temp)
}

fn emit_target(
    j: u32,
    k: usize,
    opts: &ConvertOptions,
    has_casing_transform: bool,
    should_upper_case: bool,
    to_table_idx: usize,
    to_table: &std::collections::HashMap<u32, Vec<u16>>,
    temp: &mut Vec<u16>,
) {
    if let Some(target_entries) = to_table.get(&j) {
        let mut target = if k < target_entries.len() {
            target_entries[k]
        } else {
            0
        };

        let is_upper_idx = (k % 2) == 0;

        if (opts.to_all_caps || (has_casing_transform && should_upper_case)) && !is_upper_idx {
            if k > 0 && (k - 1) < target_entries.len() {
                target = target_entries[k - 1];
            }
        } else if (opts.to_all_non_caps || (has_casing_transform && !should_upper_case)) && is_upper_idx {
            if (k + 1) < target_entries.len() {
                target = target_entries[k + 1];
            }
        }

        // Remove accents / marks if requested
        if opts.remove_mark {
            let bare_char = key_code_to_character((j & 0xFF) as u32);
            let ch = char::from_u32(bare_char as u32).unwrap_or('a');
            let is_target_upper = if opts.to_all_caps || (has_casing_transform && should_upper_case) {
                true
            } else if opts.to_all_non_caps || (has_casing_transform && !should_upper_case) {
                false
            } else {
                is_upper_idx
            };

            let out_char = if is_target_upper {
                ch.to_ascii_uppercase()
            } else {
                ch.to_ascii_lowercase()
            };
            temp.push(out_char as u16);
            return;
        }

        // Emit encoded representation
        match to_table_idx {
            0 | 1 => {
                // Unicode (0) or TCVN3 (1)
                temp.push(target);
            }
            2 | 4 => {
                // VNI Windows (2) or CP 1258 (4)
                let hi = target >> 8;
                let lo = (target as u8) as u16;
                temp.push(lo);
                if hi > 32 {
                    temp.push(hi);
                }
            }
            3 => {
                // Unicode Compound (3)
                let mark_idx = (target >> 13) as usize;
                let base = target & 0x1FFF;
                temp.push(base);
                if mark_idx > 0 && mark_idx <= UNICODE_COMPOUND_MARK.len() {
                    temp.push(UNICODE_COMPOUND_MARK[mark_idx - 1]);
                }
            }
            _ => {
                temp.push(target);
            }
        }
    }
}

// Convert contents in Windows Clipboard (CF_UNICODETEXT, CF_HTML, CF_RTF)
#[cfg(windows)]
pub fn quick_convert_clipboard(opts: &ConvertOptions) -> bool {

    unsafe {
        if OpenClipboard(std::ptr::null_mut()) == 0 {
            return false;
        }

        let mut data_unicode: Option<String> = None;
        let mut data_html: Option<String> = None;
        let mut data_rtf: Option<String> = None;

        // 1. Read CF_UNICODETEXT
        let h_uni = GetClipboardData(CF_UNICODETEXT);
        if !h_uni.is_null() {
            let ptr = GlobalLock(h_uni) as *const u16;
            if !ptr.is_null() {
                let mut len = 0;
                while *ptr.add(len) != 0 {
                    len += 1;
                }
                let slice = std::slice::from_raw_parts(ptr, len);
                data_unicode = Some(String::from_utf16_lossy(slice));
                GlobalUnlock(h_uni);
            }
        }

        // 2. Read CF_HTML
        let html_format = *CF_HTML;
        if html_format != 0 {
            let h_html = GetClipboardData(html_format);
            if !h_html.is_null() {
                let ptr = GlobalLock(h_html) as *const u8;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    data_html = Some(String::from_utf8_lossy(slice).to_string());
                    GlobalUnlock(h_html);
                }
            }
        }

        // 3. Read CF_RTF
        let rtf_format = *CF_RTF;
        if rtf_format != 0 {
            let h_rtf = GetClipboardData(rtf_format);
            if !h_rtf.is_null() {
                let ptr = GlobalLock(h_rtf) as *const u8;
                if !ptr.is_null() {
                    let mut len = 0;
                    while *ptr.add(len) != 0 {
                        len += 1;
                    }
                    let slice = std::slice::from_raw_parts(ptr, len);
                    data_rtf = Some(String::from_utf8_lossy(slice).to_string());
                    GlobalUnlock(h_rtf);
                }
            }
        }

        // Convert the extracted text
        let converted_unicode = data_unicode.map(|s| convert_util(&s, opts));
        let converted_html = data_html.map(|s| convert_util(&s, opts));
        let converted_rtf = data_rtf.map(|s| convert_util(&s, opts));

        EmptyClipboard();

        // Write converted Unicode text back
        if let Some(text) = converted_unicode {
            let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
            let byte_count = wide.len() * std::mem::size_of::<u16>();
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, byte_count);
            if !h_mem.is_null() {
                let dest = GlobalLock(h_mem) as *mut u16;
                if !dest.is_null() {
                    std::ptr::copy_nonoverlapping(wide.as_ptr(), dest, wide.len());
                    GlobalUnlock(h_mem);
                    SetClipboardData(CF_UNICODETEXT, h_mem);
                }
            }
        }

        // Write converted HTML back
        if let Some(html) = converted_html {
            let bytes = html.as_bytes();
            let byte_count = bytes.len() + 1;
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, byte_count);
            if !h_mem.is_null() {
                let dest = GlobalLock(h_mem) as *mut u8;
                if !dest.is_null() {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
                    *dest.add(bytes.len()) = 0;
                    GlobalUnlock(h_mem);
                    if html_format != 0 {
                        SetClipboardData(html_format, h_mem);
                    }
                }
            }
        }

        // Write converted RTF back
        if let Some(rtf) = converted_rtf {
            let bytes = rtf.as_bytes();
            let byte_count = bytes.len() + 1;
            let h_mem = GlobalAlloc(GMEM_MOVEABLE, byte_count);
            if !h_mem.is_null() {
                let dest = GlobalLock(h_mem) as *mut u8;
                if !dest.is_null() {
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), dest, bytes.len());
                    *dest.add(bytes.len()) = 0;
                    GlobalUnlock(h_mem);
                    if rtf_format != 0 {
                        SetClipboardData(rtf_format, h_mem);
                    }
                }
            }
        }

        CloseClipboard();
        true
    }
}

// Convert contents in macOS / Unix Clipboard using native pbpaste and pbcopy
#[cfg(not(windows))]
pub fn quick_convert_clipboard(opts: &ConvertOptions) -> bool {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let Ok(output) = Command::new("pbpaste").output() else {
        return false;
    };
    let Ok(text) = String::from_utf8(output.stdout) else {
        return false;
    };
    if text.is_empty() {
        return false;
    }

    let converted = convert_util(&text, opts);

    let Ok(mut child) = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn() else {
        return false;
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(converted.as_bytes());
    }
    let _ = child.wait();
    true
}

