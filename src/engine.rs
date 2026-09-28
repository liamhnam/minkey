// Vietnamese Input Method Engine ported from OpenKey Engine.cpp
use crate::tables::*;
use crate::types::*;

#[derive(Debug, Clone)]
pub struct VietnameseEngine {
    // Configuration settings
    pub language: u32,                  // 0: English, 1: Vietnamese
    pub input_type: InputType,          // Telex, VNI, SimpleTelex1, SimpleTelex2
    pub free_mark: bool,                // Bỏ dấu tự do (không kiểm tra ngữ pháp)
    pub code_table: usize,              // 0: Unicode, 1: TCVN3, 2: VNI, 3: Compound, 4: CP1258
    pub check_spelling: bool,           // Kiểm tra chính tả
    pub use_modern_orthography: bool,   // 0: òa, úy; 1: oà, uý
    pub quick_telex: bool,              // cc=ch, gg=gi, kk=kh, nn=ng, qq=qu, pp=ph, tt=th, uu=ươ
    pub restore_if_wrong_spelling: bool,// Tự phục hồi phím với từ sai
    pub fix_recommend_browser: bool,    // Sửa gợi ý trình duyệt
    pub use_macro: bool,                // Bật gõ tắt
    pub use_macro_in_english: bool,     // Gõ tắt trong tiếng Anh
    pub auto_caps_macro: bool,          // Tự đổi hoa/thường theo phím tắt
    pub upper_case_first_char: bool,    // Tự viết hoa chữ cái đầu
    pub allow_consonant_zfwj: bool,     // Cho phép z, w, j, f làm phụ âm đầu
    pub quick_start_consonant: bool,    // f->ph, j->gi, w->qu
    pub quick_end_consonant: bool,      // g->ng, h->nh, k->ch
    pub temp_off_spelling: bool,        // Tạm tắt kiểm tra chính tả bằng phím Ctrl
    pub temp_off_openkey: bool,         // Tạm tắt bộ gõ bằng phím Alt

    // Internal runtime state
    pub typing_word: [u32; MAX_BUFF],
    pub index: usize,
    pub key_states: [u32; MAX_BUFF],
    pub state_index: usize,
    pub long_word_helper: Vec<u32>,
    pub typing_states: Vec<Vec<u32>>,
    pub typing_states_data: Vec<u32>,
    pub special_char: Vec<u32>,
    pub space_count: usize,
    pub upper_case_status: u8,
    pub temp_disable_key: bool,
    pub will_temp_off_engine: bool,
    pub use_spell_checking_before: bool,
    pub has_handled_macro: bool,
    pub has_handle_quick_consonant: bool,
    pub macro_table: std::sync::Arc<std::sync::Mutex<crate::macro_engine::MacroTable>>,
    pub macro_key: Vec<u32>,
    pub hook_state: HookState,
}


impl Default for VietnameseEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl VietnameseEngine {
    pub fn new() -> Self {
        Self {
            language: 1, // Vietnamese
            input_type: InputType::Telex,
            free_mark: false,
            code_table: 0, // Unicode
            check_spelling: true,
            use_modern_orthography: false,
            quick_telex: false,
            restore_if_wrong_spelling: true,
            fix_recommend_browser: true,
            use_macro: true,
            use_macro_in_english: false,
            auto_caps_macro: false,
            upper_case_first_char: false,
            allow_consonant_zfwj: false,
            quick_start_consonant: false,
            quick_end_consonant: false,
            temp_off_spelling: false,
            temp_off_openkey: false,

            typing_word: [0; MAX_BUFF],
            index: 0,
            key_states: [0; MAX_BUFF],
            state_index: 0,
            long_word_helper: Vec::new(),
            typing_states: Vec::new(),
            typing_states_data: Vec::new(),
            special_char: Vec::new(),
            space_count: 0,
            upper_case_status: 0,
            temp_disable_key: false,
            will_temp_off_engine: false,
            use_spell_checking_before: true,
            has_handled_macro: false,
            has_handle_quick_consonant: false,
            macro_table: std::sync::Arc::new(std::sync::Mutex::new(crate::macro_engine::MacroTable::new())),
            macro_key: Vec::new(),
            hook_state: HookState::default(),
        }
    }

    #[inline]
    fn chr(&self, idx: usize) -> u16 {
        self.typing_word[idx] as u16
    }

    pub fn start_new_session(&mut self) {
        self.index = 0;
        self.hook_state.backspace_count = 0;
        self.hook_state.new_char_count = 0;
        self.temp_disable_key = false;
        self.state_index = 0;
        self.has_handled_macro = false;
        self.has_handle_quick_consonant = false;
        self.macro_key.clear();
        self.long_word_helper.clear();
    }


    pub fn is_word_break(&self, event: KeyEvent, key_code: u16) -> bool {
        if event == KeyEvent::Mouse {
            return true;
        }
        let break_codes = [
            KEY_ESC, KEY_TAB, KEY_ENTER, KEY_RETURN, KEY_LEFT, KEY_RIGHT, KEY_DOWN, KEY_UP,
            KEY_COMMA, KEY_DOT, KEY_SLASH, KEY_SEMICOLON, KEY_QUOTE, KEY_BACK_SLASH,
            KEY_MINUS, KEY_EQUALS, KEY_BACKQUOTE,
            VK_INSERT, VK_HOME, VK_END, VK_DELETE_SYS, VK_PRIOR, VK_NEXT,
            VK_SNAPSHOT, VK_PRINT, VK_SELECT, VK_HELP, VK_EXECUTE, VK_NUMLOCK, VK_SCROLL,
        ];
        break_codes.contains(&key_code)
    }

    pub fn is_char_key_code(key_code: u16) -> bool {
        let char_key_codes = [
            KEY_BACKQUOTE, KEY_1, KEY_2, KEY_3, KEY_4, KEY_5, KEY_6, KEY_7, KEY_8, KEY_9, KEY_0,
            KEY_MINUS, KEY_EQUALS, KEY_LEFT_BRACKET, KEY_RIGHT_BRACKET, KEY_BACK_SLASH,
            KEY_SEMICOLON, KEY_QUOTE, KEY_COMMA, KEY_DOT, KEY_SLASH,
        ];
        char_key_codes.contains(&key_code)
    }

    pub fn is_macro_break_code(key_code: u16) -> bool {
        let macro_break_codes = [
            KEY_RETURN, KEY_COMMA, KEY_DOT, KEY_SLASH, KEY_SEMICOLON,
            KEY_QUOTE, KEY_BACK_SLASH, KEY_MINUS, KEY_EQUALS,
        ];
        macro_break_codes.contains(&key_code)
    }

    fn set_key_data(&mut self, index: usize, key_code: u16, is_caps: bool) {
        if index < MAX_BUFF {
            self.typing_word[index] = (key_code as u32) | if is_caps { CAPS_MASK } else { 0 };
        }
    }

    fn insert_key(&mut self, key_code: u16, is_caps: bool, check_spelling: bool) {
        if self.index >= MAX_BUFF {
            self.long_word_helper.push(self.typing_word[0]);
            for i in 0..(MAX_BUFF - 1) {
                self.typing_word[i] = self.typing_word[i + 1];
            }
            self.set_key_data(self.index - 1, key_code, is_caps);
        } else {
            self.set_key_data(self.index, key_code, is_caps);
            self.index += 1;
        }

        if self.check_spelling && check_spelling {
            self.check_spelling_internal(false);
        }

        // Allow 'd' after consonant
        if key_code == KEY_D && self.index >= 2 && is_consonant(self.chr(self.index - 2)) {
            self.temp_disable_key = false;
        }
    }

    fn insert_state(&mut self, key_code: u16, is_caps: bool) {
        if self.state_index >= MAX_BUFF {
            for i in 0..(MAX_BUFF - 1) {
                self.key_states[i] = self.key_states[i + 1];
            }
            self.key_states[self.state_index - 1] = (key_code as u32) | if is_caps { CAPS_MASK } else { 0 };
        } else {
            self.key_states[self.state_index] = (key_code as u32) | if is_caps { CAPS_MASK } else { 0 };
            self.state_index += 1;
        }
    }

    fn save_word(&mut self) {
        if self.hook_state.code != HookCodeState::ReplaceMacro {
            if self.index > 0 {
                if !self.long_word_helper.is_empty() {
                    self.typing_states_data.clear();
                    for (i, &ch) in self.long_word_helper.iter().enumerate() {
                        if i != 0 && i % MAX_BUFF == 0 {
                            self.typing_states.push(self.typing_states_data.clone());
                            self.typing_states_data.clear();
                        }
                        self.typing_states_data.push(ch);
                    }
                    self.typing_states.push(self.typing_states_data.clone());
                    self.long_word_helper.clear();
                }

                self.typing_states_data.clear();
                for i in 0..self.index {
                    self.typing_states_data.push(self.typing_word[i]);
                }
                self.typing_states.push(self.typing_states_data.clone());
            }
        } else {
            self.typing_states_data.clear();
            for (i, &code) in self.hook_state.macro_data.iter().enumerate() {
                if i != 0 && i % MAX_BUFF == 0 {
                    self.typing_states.push(self.typing_states_data.clone());
                    self.typing_states_data.clear();
                }
                self.typing_states_data.push(code);
            }
            self.typing_states.push(self.typing_states_data.clone());
        }
    }

    fn save_word_repetition(&mut self, key_code: u32, count: usize) {
        self.typing_states_data.clear();
        for _ in 0..count {
            self.typing_states_data.push(key_code);
        }
        self.typing_states.push(self.typing_states_data.clone());
    }

    fn save_special_char(&mut self) {
        self.typing_states_data.clear();
        for &c in &self.special_char {
            self.typing_states_data.push(c);
        }
        self.typing_states.push(self.typing_states_data.clone());
        self.special_char.clear();
    }

    fn restore_last_typing_state(&mut self) {
        if let Some(data) = self.typing_states.pop() {
            self.typing_states_data = data;
            if !self.typing_states_data.is_empty() {
                if self.typing_states_data[0] == KEY_SPACE as u32 {
                    self.space_count = self.typing_states_data.len();
                    self.index = 0;
                } else if Self::is_char_key_code(self.typing_states_data[0] as u16) {
                    self.index = 0;
                    self.special_char = self.typing_states_data.clone();
                    self.check_spelling_internal(false);
                } else {
                    for (i, &val) in self.typing_states_data.iter().enumerate() {
                        self.typing_word[i] = val;
                    }
                    self.index = self.typing_states_data.len();
                }
            }
        }
    }

    pub fn check_spelling_internal(&mut self, force_check_vowel: bool) {
        let mut spelling_ok = false;
        let mut spelling_vowel_ok = true;
        let mut spelling_end_index = self.index;

        if self.index > 0 && self.chr(self.index - 1) == KEY_RIGHT_BRACKET {
            spelling_end_index = self.index - 1;
        }

        if spelling_end_index > 0 {
            let mut j = 0;
            // Check first consonant
            if is_consonant(self.chr(0)) {
                let consonant_tbl = &*CONSONANT_TABLE;
                for row in consonant_tbl.iter() {
                    let mut flag = false;
                    if spelling_end_index < row.len() {
                        flag = true;
                    }
                    for (col_idx, &code) in row.iter().enumerate() {
                        if spelling_end_index > col_idx {
                            let end_mask = if self.quick_start_consonant { END_CONSONANT_MASK } else { 0 };
                            let allow_mask = if self.allow_consonant_zfwj { CONSONANT_ALLOW_MASK } else { 0 };
                            if (code & !end_mask) != self.chr(col_idx) && (code & !allow_mask) != self.chr(col_idx) {
                                flag = true;
                                break;
                            }
                        }
                    }
                    if flag {
                        continue;
                    }
                    j = row.len();
                    break;
                }
            }

            if j == spelling_end_index {
                spelling_ok = true;
            }

            // Check next vowel
            let mut k = j;
            if k < self.index && self.chr(k) == KEY_U && k > 0 && k < spelling_end_index - 1 && self.chr(k - 1) == KEY_Q {
                k += 1;
                j = k;
            } else if self.index >= 2 && self.chr(0) == KEY_G && self.chr(1) == KEY_I && (self.index > 2 && is_consonant(self.chr(2))) {
                j = 1;
                k = 1;
            }

            for _ in 0..3 {
                if k < spelling_end_index && !is_consonant(self.chr(k)) {
                    k += 1;
                }
            }

            if k > j {
                spelling_vowel_ok = false;
                if k - j > 1 && force_check_vowel {
                    if let Some(vowel_set) = VOWEL_COMBINE.get(&self.chr(j)) {
                        for row in vowel_set {
                            let mut flag = false;
                            let mut ii = 1;
                            while ii < row.len() {
                                if j + ii - 1 < spelling_end_index {
                                    let current_char_with_tone = (self.chr(j + ii - 1) as u32)
                                        | (self.typing_word[j + ii - 1] & (TONEW_MASK | TONE_MASK));
                                    if row[ii] != current_char_with_tone {
                                        flag = true;
                                        break;
                                    }
                                }
                                ii += 1;
                            }
                            if flag || (k < spelling_end_index && row[0] == 0) || (j + ii - 1 < spelling_end_index && !is_consonant(self.chr(j + ii - 1))) {
                                continue;
                            }
                            spelling_vowel_ok = true;
                            break;
                        }
                    }
                } else if !is_consonant(self.chr(j)) {
                    spelling_vowel_ok = true;
                }

                // Check last consonant
                let end_consonants = &*END_CONSONANT_TABLE;
                for row in end_consonants.iter() {
                    let mut flag = false;
                    let mut cj = 0;
                    while cj < row.len() {
                        if spelling_end_index > k + cj {
                            let end_mask = if self.quick_end_consonant { END_CONSONANT_MASK } else { 0 };
                            if (row[cj] & !end_mask) != self.chr(k + cj) {
                                flag = true;
                                break;
                            }
                        }
                        cj += 1;
                    }
                    if flag {
                        continue;
                    }
                    if k + cj >= spelling_end_index {
                        spelling_ok = true;
                        break;
                    }
                }

                // Limit: end consonant "ch", "t" cannot use with "~", "`", "?"
                if spelling_ok {
                    if self.index >= 3 && self.chr(self.index - 1) == KEY_H && self.chr(self.index - 2) == KEY_C {
                        let prev_mask = self.typing_word[self.index - 3];
                        if !((prev_mask & MARK1_MASK) != 0 || (prev_mask & MARK5_MASK) != 0 || (prev_mask & MARK_MASK) == 0) {
                            spelling_ok = false;
                        }
                    } else if self.index >= 2 && self.chr(self.index - 1) == KEY_T {
                        let prev_mask = self.typing_word[self.index - 2];
                        if !((prev_mask & MARK1_MASK) != 0 || (prev_mask & MARK5_MASK) != 0 || (prev_mask & MARK_MASK) == 0) {
                            spelling_ok = false;
                        }
                    }
                }
            }
        } else {
            spelling_ok = true;
        }

        self.temp_disable_key = !(spelling_ok && spelling_vowel_ok);
    }

    fn find_and_calculate_vowel(&self, for_grammar: bool) -> (usize, usize, usize) {
        let mut vowel_count: usize = 0;
        let mut vsi = 0;
        let mut vei = 0;
        if self.index == 0 {
            return (0, 0, 0);
        }
        for i in (0..self.index).rev() {
            if is_consonant(self.chr(i)) {
                if vowel_count > 0 {
                    break;
                }
            } else {
                if vowel_count == 0 {
                    vei = i;
                }
                if !for_grammar {
                    if i >= 1 && ((self.chr(i) == KEY_I && self.chr(i - 1) == KEY_G)
                        || (self.chr(i) == KEY_U && self.chr(i - 1) == KEY_Q)) {
                        break;
                    }
                }
                vsi = i;
                vowel_count += 1;
            }
        }
        if vsi >= 1 && self.chr(vsi) == KEY_U && self.chr(vsi - 1) == KEY_Q {
            vsi += 1;
            vowel_count = vowel_count.saturating_sub(1);
        }
        (vsi, vei, vowel_count)
    }

    fn can_has_end_consonant(&self, vsi: usize, vei: usize) -> bool {
        if let Some(vo) = VOWEL_COMBINE.get(&self.chr(vsi)) {
            for row in vo {
                let mut kk = vsi;
                let mut matches = true;
                for ii in 1..row.len() {
                    if kk > vei || ((self.chr(kk) as u32 | (self.typing_word[kk] & (TONE_MASK | TONEW_MASK))) != row[ii]) {
                        matches = false;
                        break;
                    }
                    kk += 1;
                }
                if matches {
                    return row[0] == 1;
                }
            }
        }
        false
    }

    fn handle_modern_mark(&self, vsi: usize, vei: usize, vowel_count: usize) -> (usize, u8) {
        let mut vwsm = vei;
        let mut bpc = (self.index - vei) as u8;

        if vowel_count == 3 && ((self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_A && self.chr(vsi + 2) == KEY_I)
            || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_Y && self.chr(vsi + 2) == KEY_U)
            || (self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_E && self.chr(vsi + 2) == KEY_O)
            || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_Y && self.chr(vsi + 2) == KEY_A)) {
            vwsm = vsi + 1;
            bpc = (self.index - vwsm) as u8;
        } else if (self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_I)
            || (self.chr(vsi) == KEY_A && self.chr(vsi + 1) == KEY_I)
            || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_I) {
            vwsm = vsi;
            bpc = (self.index - vwsm) as u8;
        } else if vei >= 1 && self.chr(vei - 1) == KEY_A && self.chr(vei) == KEY_Y {
            vwsm = vei - 1;
            bpc = ((self.index - vei) + 1) as u8;
        } else if vsi + 1 < self.index && self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_O {
            vwsm = vsi + 1;
            bpc = (self.index - vwsm) as u8;
        } else if vsi + 1 < self.index && (self.chr(vsi + 1) == KEY_O || self.chr(vsi + 1) == KEY_U) {
            vwsm = vei.saturating_sub(1);
            bpc = ((self.index - vei) + 1) as u8;
        } else if self.chr(vsi) == KEY_O || self.chr(vsi) == KEY_U {
            vwsm = vei;
            bpc = (self.index - vei) as u8;
        }

        // Rule 3.1
        if (self.chr(vsi) == KEY_I && (self.typing_word[vsi + 1] & (KEY_E as u32 | TONE_MASK)) != 0)
            || (self.chr(vsi) == KEY_Y && (self.typing_word[vsi + 1] & (KEY_E as u32 | TONE_MASK)) != 0)
            || (self.chr(vsi) == KEY_U && (self.typing_word[vsi + 1] == (KEY_O as u32 | TONE_MASK)))
            || ((self.typing_word[vsi] == (KEY_U as u32 | TONEW_MASK)) && (self.typing_word[vsi + 1] == (KEY_O as u32 | TONEW_MASK))) {
            if vsi + 2 < self.index {
                let c = self.chr(vsi + 2);
                if matches!(c, KEY_P | KEY_T | KEY_M | KEY_N | KEY_O | KEY_U | KEY_I | KEY_C)
                    || (vsi + 3 < self.index && c == KEY_C && self.chr(vsi + 3) == KEY_H)
                    || (vsi + 3 < self.index && c == KEY_N && self.chr(vsi + 3) == KEY_H)
                    || (vsi + 3 < self.index && c == KEY_N && self.chr(vsi + 3) == KEY_G) {
                    vwsm = vsi + 1;
                    bpc = (self.index - vwsm) as u8;
                } else {
                    vwsm = vsi;
                    bpc = (self.index - vwsm) as u8;
                }
            } else {
                vwsm = vsi;
                bpc = (self.index - vwsm) as u8;
            }
        }
        // Rule 3.2
        else if (self.chr(vsi) == KEY_I && self.chr(vsi + 1) == KEY_A)
            || (self.chr(vsi) == KEY_Y && self.chr(vsi + 1) == KEY_A)
            || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_A)
            || (self.chr(vsi) == KEY_U && (self.typing_word[vsi + 1] == (KEY_U as u32 | TONEW_MASK))) {
            vwsm = vsi;
            bpc = (self.index - vwsm) as u8;
        }

        // Rule 4
        if vowel_count == 2 {
            if (self.chr(vsi) == KEY_I && self.chr(vsi + 1) == KEY_A)
                || (self.chr(vsi) == KEY_I && self.chr(vsi + 1) == KEY_U)
                || (self.chr(vsi) == KEY_I && self.chr(vsi + 1) == KEY_O) {
                if vsi == 0 || self.chr(vsi - 1) != KEY_G {
                    vwsm = vsi;
                    bpc = (self.index - vwsm) as u8;
                } else {
                    vwsm = vsi + 1;
                    bpc = (self.index - vwsm) as u8;
                }
            } else if self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_A {
                if vsi == 0 || self.chr(vsi - 1) != KEY_Q {
                    if vei + 1 >= self.index || !self.can_has_end_consonant(vsi, vei) {
                        vwsm = vsi;
                        bpc = (self.index - vwsm) as u8;
                    }
                } else {
                    vwsm = vsi + 1;
                    bpc = (self.index - vwsm) as u8;
                }
            } else if self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_O {
                vwsm = vei;
                bpc = (self.index - vwsm) as u8;
            }
        }

        (vwsm, bpc)
    }

    fn handle_old_mark(&self, vsi: usize, vei: usize, vowel_count: usize) -> (usize, u8) {
        let mut vwsm = if vowel_count == 0 && self.chr(vei) == KEY_I {
            vei
        } else {
            vsi
        };
        let mut bpc = (self.index - vwsm) as u8;

        if vowel_count == 3 || (vei + 1 < self.index && is_consonant(self.chr(vei + 1)) && self.can_has_end_consonant(vsi, vei)) {
            vwsm = vsi + 1;
            bpc = (self.index - vwsm) as u8;
        }

        for ii in vsi..=vei {
            if (self.chr(ii) == KEY_E && (self.typing_word[ii] & TONE_MASK) != 0)
                || (self.chr(ii) == KEY_O && (self.typing_word[ii] & TONEW_MASK) != 0) {
                vwsm = ii;
                bpc = (self.index - vwsm) as u8;
                break;
            }
        }

        (vwsm, bpc)
    }

    fn insert_mark(&mut self, mark_mask: u32, can_modify_flag: bool) {
        if can_modify_flag {
            self.hook_state.code = HookCodeState::WillProcess;
        }
        self.hook_state.backspace_count = 0;
        self.hook_state.new_char_count = 0;

        let (vsi, vei, vowel_count) = self.find_and_calculate_vowel(false);
        if vowel_count == 0 {
            return;
        }

        let (vwsm, mut bpc) = if vowel_count == 1 {
            (vei, (self.index - vei) as u8)
        } else {
            let res = if !self.use_modern_orthography {
                self.handle_old_mark(vsi, vei, vowel_count)
            } else {
                self.handle_modern_mark(vsi, vei, vowel_count)
            };
            if (self.typing_word[vei] & TONE_MASK) != 0 || (self.typing_word[vei] & TONEW_MASK) != 0 {
                (vei, res.1)
            } else {
                res
            }
        };

        let mut kk = self.index.saturating_sub(1).saturating_sub(vsi);

        // If duplicate same mark -> restore
        if (self.typing_word[vwsm] & mark_mask) != 0 {
            self.typing_word[vwsm] &= !MARK_MASK;
            if can_modify_flag {
                self.hook_state.code = HookCodeState::Restore;
            }
            for ii in vsi..self.index {
                self.typing_word[ii] &= !MARK_MASK;
                self.hook_state.char_data[kk] = get_character_code(self.typing_word[ii], self.code_table);
                kk = kk.saturating_sub(1);
            }
            self.temp_disable_key = true;
        } else {
            self.typing_word[vwsm] &= !MARK_MASK;
            self.typing_word[vwsm] |= mark_mask;
            for ii in vsi..self.index {
                if ii != vwsm {
                    self.typing_word[ii] &= !MARK_MASK;
                }
                self.hook_state.char_data[kk] = get_character_code(self.typing_word[ii], self.code_table);
                kk = kk.saturating_sub(1);
            }
            bpc = (self.index - vsi) as u8;
        }

        self.hook_state.backspace_count = bpc;
        self.hook_state.new_char_count = bpc;
    }

    fn remove_mark(&mut self) {
        let (vsi, vei, _) = self.find_and_calculate_vowel(true);
        let mut is_changed = false;
        if self.index > 0 {
            for i in vsi..=vei {
                if (self.typing_word[i] & MARK_MASK) != 0 {
                    self.typing_word[i] &= !MARK_MASK;
                    is_changed = true;
                }
            }
        }
        if is_changed {
            self.hook_state.code = HookCodeState::WillProcess;
            self.hook_state.backspace_count = 0;
            for i in (vsi..self.index).rev() {
                self.hook_state.backspace_count += 1;
                self.hook_state.char_data[self.index - 1 - i] = get_character_code(self.typing_word[i], self.code_table);
            }
            self.hook_state.new_char_count = self.hook_state.backspace_count;
        } else {
            self.hook_state.code = HookCodeState::DoNothing;
        }
    }

    fn check_correct_vowel(&self, charset: &[Vec<u16>], row_idx: usize, mark_key: u16) -> bool {
        if self.index >= 2 && self.chr(self.index - 1) == KEY_U && self.chr(self.index - 2) == KEY_Q {
            return false;
        }
        let mut k = self.index as i32 - 1;
        let row = &charset[row_idx];
        for j in (0..row.len()).rev() {
            let end_mask = if self.quick_end_consonant { END_CONSONANT_MASK } else { 0 };
            if (row[j] & !end_mask) != self.chr(k as usize) {
                return false;
            }
            k -= 1;
            if k < 0 {
                break;
            }
        }

        // Limit mark for end consonant: "C", "T"
        if row.len() > 1 && (self.is_key_f(mark_key) || self.is_key_x(mark_key) || self.is_key_r(mark_key)) {
            if row[1] == KEY_C || row[1] == KEY_T {
                return false;
            } else if row.len() > 2 && row[2] == KEY_T {
                return false;
            }
        }

        if k >= 0 && self.chr(k as usize) == self.chr((k + 1) as usize) {
            return false;
        }

        true
    }


    fn insert_d(&mut self) {
        self.hook_state.code = HookCodeState::WillProcess;
        self.hook_state.backspace_count = 0;
        for ii in (0..self.index).rev() {
            self.hook_state.backspace_count += 1;
            if self.chr(ii) == KEY_D {
                if (self.typing_word[ii] & TONE_MASK) != 0 {
                    self.hook_state.code = HookCodeState::Restore;
                    self.typing_word[ii] &= !TONE_MASK;
                    self.hook_state.char_data[self.index - 1 - ii] = self.typing_word[ii];
                    self.temp_disable_key = true;
                    break;
                } else {
                    self.typing_word[ii] |= TONE_MASK;
                    self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
                }
                break;
            } else {
                self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
            }
        }
        self.hook_state.new_char_count = self.hook_state.backspace_count;
    }

    fn insert_aoe(&mut self, data: u16) {
        let (vsi, vei, _) = self.find_and_calculate_vowel(false);
        for ii in vsi..=vei {
            self.typing_word[ii] &= !TONEW_MASK;
        }

        self.hook_state.code = HookCodeState::WillProcess;
        self.hook_state.backspace_count = 0;

        for ii in (0..self.index).rev() {
            self.hook_state.backspace_count += 1;
            if self.chr(ii) == data {
                if (self.typing_word[ii] & TONE_MASK) != 0 {
                    self.hook_state.code = HookCodeState::Restore;
                    self.typing_word[ii] &= !TONE_MASK;
                    self.hook_state.char_data[self.index - 1 - ii] = self.typing_word[ii];
                    if data != KEY_O {
                        self.temp_disable_key = true;
                    }
                    break;
                } else {
                    self.typing_word[ii] |= TONE_MASK;
                    if data != KEY_D {
                        self.typing_word[ii] &= !TONEW_MASK;
                    }
                    self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
                }
                break;
            } else {
                self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
            }
        }
        self.hook_state.new_char_count = self.hook_state.backspace_count;
    }

    fn insert_w(&mut self) {
        let (vsi, vei, vowel_count) = self.find_and_calculate_vowel(false);
        for ii in vsi..=vei {
            self.typing_word[ii] &= !TONE_MASK;
        }

        if vowel_count > 1 {
            self.hook_state.backspace_count = (self.index - vsi) as u8;
            self.hook_state.new_char_count = self.hook_state.backspace_count;

            if (((self.typing_word[vsi] & TONEW_MASK) != 0) && ((self.typing_word[vsi + 1] & TONEW_MASK) != 0))
                || (((self.typing_word[vsi] & TONEW_MASK) != 0) && self.chr(vsi + 1) == KEY_I)
                || (((self.typing_word[vsi] & TONEW_MASK) != 0) && self.chr(vsi + 1) == KEY_A) {
                self.hook_state.code = HookCodeState::Restore;
                for ii in vsi..self.index {
                    self.typing_word[ii] &= !TONEW_MASK;
                    self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table) & !STANDALONE_MASK;
                }
                self.temp_disable_key = true;
            } else {
                self.hook_state.code = HookCodeState::WillProcess;
                if self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_O {
                    if vsi >= 2 && self.chr(vsi - 2) == KEY_T && self.chr(vsi - 1) == KEY_H {
                        self.typing_word[vsi + 1] |= TONEW_MASK;
                        if vsi + 2 < self.index && self.chr(vsi + 2) == KEY_N {
                            self.typing_word[vsi] |= TONEW_MASK;
                        }
                    } else if vsi >= 1 && self.chr(vsi - 1) == KEY_Q {
                        self.typing_word[vsi + 1] |= TONEW_MASK;
                    } else {
                        self.typing_word[vsi] |= TONEW_MASK;
                        self.typing_word[vsi + 1] |= TONEW_MASK;
                    }
                } else if (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_A)
                    || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_I)
                    || (self.chr(vsi) == KEY_U && self.chr(vsi + 1) == KEY_U)
                    || (self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_I) {
                    self.typing_word[vsi] |= TONEW_MASK;
                } else if (self.chr(vsi) == KEY_I && self.chr(vsi + 1) == KEY_O)
                    || (self.chr(vsi) == KEY_O && self.chr(vsi + 1) == KEY_A) {
                    self.typing_word[vsi + 1] |= TONEW_MASK;
                } else {
                    self.temp_disable_key = true;
                    self.hook_state.code = HookCodeState::DoNothing;
                }

                for ii in vsi..self.index {
                    self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
                }
            }
            return;
        }

        self.hook_state.code = HookCodeState::WillProcess;
        self.hook_state.backspace_count = 0;

        for ii in (0..self.index).rev() {
            if ii < vsi {
                break;
            }
            self.hook_state.backspace_count += 1;
            match self.chr(ii) {
                KEY_A | KEY_U | KEY_O => {
                    if (self.typing_word[ii] & TONEW_MASK) != 0 {
                        if (self.typing_word[ii] & STANDALONE_MASK) != 0 {
                            self.hook_state.code = HookCodeState::WillProcess;
                            if self.chr(ii) == KEY_U {
                                self.typing_word[ii] = (KEY_W as u32) | if (self.typing_word[ii] & CAPS_MASK) != 0 { CAPS_MASK } else { 0 };
                            } else if self.chr(ii) == KEY_O {
                                self.hook_state.code = HookCodeState::Restore;
                                self.typing_word[ii] = (KEY_O as u32) | if (self.typing_word[ii] & CAPS_MASK) != 0 { CAPS_MASK } else { 0 };
                            }
                            self.hook_state.char_data[self.index - 1 - ii] = self.typing_word[ii];
                        } else {
                            self.hook_state.code = HookCodeState::Restore;
                            self.typing_word[ii] &= !TONEW_MASK;
                            self.hook_state.char_data[self.index - 1 - ii] = self.typing_word[ii];
                        }
                        self.temp_disable_key = true;
                    } else {
                        self.typing_word[ii] |= TONEW_MASK;
                        self.typing_word[ii] &= !TONE_MASK;
                        self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
                    }
                }
                _ => {
                    self.hook_state.char_data[self.index - 1 - ii] = get_character_code(self.typing_word[ii], self.code_table);
                }
            }
        }
        self.hook_state.new_char_count = self.hook_state.backspace_count;
    }

    fn reverse_last_standalone_char(&mut self, key_code: u32, is_caps: bool) {
        self.hook_state.code = HookCodeState::WillProcess;
        self.hook_state.backspace_count = 0;
        self.hook_state.new_char_count = 1;
        self.hook_state.ext_code = 4;
        self.typing_word[self.index - 1] = key_code | TONEW_MASK | STANDALONE_MASK | if is_caps { CAPS_MASK } else { 0 };
        self.hook_state.char_data[0] = get_character_code(self.typing_word[self.index - 1], self.code_table);
    }

    fn check_for_standalone_char(&mut self, data: u16, is_caps: bool, key_will_reverse: u32) {
        if self.index > 0 && (self.chr(self.index - 1) as u32) == key_will_reverse && (self.typing_word[self.index - 1] & TONEW_MASK) != 0 {
            self.hook_state.code = HookCodeState::WillProcess;
            self.hook_state.backspace_count = 1;
            self.hook_state.new_char_count = 1;
            self.typing_word[self.index - 1] = (data as u32) | if is_caps { CAPS_MASK } else { 0 };
            self.hook_state.char_data[0] = get_character_code(self.typing_word[self.index - 1], self.code_table);
            return;
        }

        if self.index > 0 && self.chr(self.index - 1) == KEY_U && key_will_reverse == KEY_O as u32 {
            self.insert_key(key_will_reverse as u16, is_caps, true);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
            return;
        }

        if self.index == 0 {
            self.insert_key(data, is_caps, false);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
            return;
        } else if self.index == 1 {
            for &bad in STANDALONE_W_BAD.iter() {
                if self.chr(0) == bad {
                    self.insert_key(data, is_caps, true);
                    return;
                }
            }
            self.insert_key(data, is_caps, false);
            self.reverse_last_standalone_char(key_will_reverse, is_caps);
            return;
        } else if self.index == 2 {
            for pair in DOUBLE_W_ALLOWED.iter() {
                if self.chr(0) == pair[0] && self.chr(1) == pair[1] {
                    self.insert_key(data, is_caps, false);
                    self.reverse_last_standalone_char(key_will_reverse, is_caps);
                    return;
                }
            }
            self.insert_key(data, is_caps, true);
            return;
        }

        self.insert_key(data, is_caps, true);
    }

    fn check_grammar(&mut self, delta_backspace: i32) {
        if self.index <= 1 || self.index >= MAX_BUFF {
            return;
        }
        let (vsi, vei, vowel_count) = self.find_and_calculate_vowel(true);
        if vowel_count == 0 {
            return;
        }

        let mut is_checked_grammar = false;

        // "thuơn", "ưoi", "ưom", "ưoc"
        if self.index >= 3 {
            for i in (0..self.index).rev() {
                let c = self.chr(i);
                if matches!(c, KEY_N | KEY_C | KEY_I | KEY_M | KEY_P | KEY_T) {
                    if i >= 2 && self.chr(i - 1) == KEY_O && self.chr(i - 2) == KEY_U {
                        let w1 = self.typing_word[i - 1] & TONEW_MASK;
                        let w2 = self.typing_word[i - 2] & TONEW_MASK;
                        if (w1 != 0) ^ (w2 != 0) {
                            self.typing_word[i - 2] |= TONEW_MASK;
                            self.typing_word[i - 1] |= TONEW_MASK;
                            is_checked_grammar = true;
                            break;
                        }
                    }
                }
            }
        }

        // Check mark repositioning
        if self.index >= 2 {
            for i in vsi..=vei {
                if (self.typing_word[i] & MARK_MASK) != 0 {
                    let mark = self.typing_word[i] & MARK_MASK;
                    self.typing_word[i] &= !MARK_MASK;
                    self.insert_mark(mark, false);
                    is_checked_grammar = true;
                    break;
                }
            }
        }

        if is_checked_grammar {
            if self.hook_state.code == HookCodeState::DoNothing {
                self.hook_state.code = HookCodeState::WillProcess;
            }
            self.hook_state.backspace_count = 0;
            for i in (vsi..self.index).rev() {
                self.hook_state.backspace_count += 1;
                self.hook_state.char_data[self.index - 1 - i] = get_character_code(self.typing_word[i], self.code_table);
            }
            self.hook_state.new_char_count = self.hook_state.backspace_count;
            self.hook_state.backspace_count = (self.hook_state.backspace_count as i32 + delta_backspace).max(0) as u8;
            self.hook_state.ext_code = 4;
        }
    }

    fn check_restore_if_wrong_spelling(&mut self, handle_code: HookCodeState) -> bool {
        for ii in 0..self.index {
            if !is_consonant(self.chr(ii))
                && ((self.typing_word[ii] & (MARK_MASK | TONE_MASK | TONEW_MASK)) != 0) {
                self.hook_state.code = handle_code;
                self.hook_state.backspace_count = self.index as u8;
                self.hook_state.new_char_count = self.state_index as u8;
                for i in 0..self.state_index {
                    self.typing_word[i] = self.key_states[i];
                    self.hook_state.char_data[self.state_index - 1 - i] = self.typing_word[i];
                }
                self.index = self.state_index;
                return true;
            }
        }
        false
    }

    fn handle_quick_telex(&mut self, data: u16, is_caps: bool) {
        if let Some(&pair) = QUICK_TELEX.get(&(data as u32)) {
            self.hook_state.code = HookCodeState::WillProcess;
            self.hook_state.backspace_count = 1;
            self.hook_state.new_char_count = 2;
            let caps_flag = if is_caps { CAPS_MASK } else { 0 };
            self.hook_state.char_data[1] = (pair[0] as u32) | caps_flag;
            self.hook_state.char_data[0] = (pair[1] as u32) | caps_flag;
            self.insert_key(pair[1], is_caps, false);
        }
    }

    fn check_quick_consonant(&mut self) -> bool {
        if self.index <= 1 {
            return false;
        }
        let mut l = 0;
        if self.index > 0 {
            if self.quick_start_consonant && QUICK_START_CONSONANT.contains_key(&self.chr(0)) {
                let pair = QUICK_START_CONSONANT[&self.chr(0)];
                self.hook_state.code = HookCodeState::Restore;
                self.hook_state.backspace_count = self.index as u8;
                self.hook_state.new_char_count = (self.index + 1) as u8;
                if self.index < MAX_BUFF - 1 {
                    self.index += 1;
                }
                for i in (2..self.index).rev() {
                    self.typing_word[i] = self.typing_word[i - 1];
                }
                let caps0 = (self.typing_word[0] & CAPS_MASK) != 0;
                let caps2 = (self.typing_word[2] & CAPS_MASK) != 0;
                self.typing_word[1] = (pair[1] as u32) | if caps0 && caps2 { CAPS_MASK } else { 0 };
                self.typing_word[0] = (pair[0] as u32) | if caps0 { CAPS_MASK } else { 0 };
                l = 1;
            }

            if self.quick_end_consonant
                && (self.index >= 2 && !is_consonant(self.chr(self.index - 2)))
                && QUICK_END_CONSONANT.contains_key(&self.chr(self.index - 1)) {
                let pair = QUICK_END_CONSONANT[&self.chr(self.index - 1)];
                self.hook_state.code = HookCodeState::Restore;
                if l == 1 {
                    self.hook_state.new_char_count += 1;
                } else {
                    self.hook_state.backspace_count = 1;
                    self.hook_state.new_char_count = 2;
                }
                if self.index < MAX_BUFF - 1 {
                    self.index += 1;
                }
                let caps = (self.typing_word[self.index - 2] & CAPS_MASK) != 0;
                self.typing_word[self.index - 1] = (pair[1] as u32) | if caps { CAPS_MASK } else { 0 };
                self.typing_word[self.index - 2] = (pair[0] as u32) | if caps { CAPS_MASK } else { 0 };
                l = 1;
            }

            if l == 1 {
                self.has_handle_quick_consonant = true;
                for i in (0..self.index).rev() {
                    self.hook_state.char_data[self.index - 1 - i] = get_character_code(self.typing_word[i], self.code_table);
                }
                return true;
            }
        }
        false
    }

    fn upper_case_first_character(&mut self) {
        if (self.typing_word[0] & CAPS_MASK) == 0 {
            self.hook_state.code = HookCodeState::WillProcess;
            self.hook_state.backspace_count = 0;
            self.hook_state.new_char_count = 1;
            self.typing_word[0] |= CAPS_MASK;
            self.hook_state.char_data[0] = get_character_code(self.typing_word[0], self.code_table);
            self.upper_case_status = 0;
            if self.use_macro && !self.hook_state.macro_key.is_empty() {
                self.hook_state.macro_key[0] |= CAPS_MASK;
            }
        }
    }

    #[inline]
    fn is_key_z(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_0,
            _ => key == KEY_Z,
        }
    }

    #[inline]
    fn is_key_d(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_9,
            _ => key == KEY_D,
        }
    }

    #[inline]
    fn is_key_w(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_7 || key == KEY_8,
            _ => key == KEY_W,
        }
    }

    #[inline]
    fn is_key_double(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_6,
            _ => key == KEY_A || key == KEY_O || key == KEY_E,
        }
    }

    #[inline]
    fn is_key_s(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_1,
            _ => key == KEY_S,
        }
    }

    #[inline]
    fn is_key_f(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_2,
            _ => key == KEY_F,
        }
    }

    #[inline]
    fn is_key_r(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_3,
            _ => key == KEY_R,
        }
    }

    #[inline]
    fn is_key_x(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_4,
            _ => key == KEY_X,
        }
    }

    #[inline]
    fn is_key_j(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => key == KEY_5,
            _ => key == KEY_J,
        }
    }

    #[inline]
    fn is_mark_key(&self, key: u16) -> bool {
        match self.input_type {
            InputType::Vni => matches!(key, KEY_1 | KEY_2 | KEY_3 | KEY_4 | KEY_5),
            _ => matches!(key, KEY_S | KEY_F | KEY_R | KEY_X | KEY_J),
        }
    }

    fn handle_main_key(&mut self, data: u16, is_caps: bool) {
        // If Z key (Telex 'z', VNI '0'), remove mark
        if self.is_key_z(data) {
            let before = self.typing_word;
            self.remove_mark();
            if self.typing_word == before {
                self.insert_key(data, is_caps, true);
            }
            return;
        }

        // Standalone key [
        if data == KEY_LEFT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_O as u32);
            return;
        }

        // Standalone key ]
        if data == KEY_RIGHT_BRACKET {
            self.check_for_standalone_char(data, is_caps, KEY_U as u32);
            return;
        }

        // If D key (Telex 'd', VNI '9')
        if self.is_key_d(data) {
            let mut is_correct;
            let consonant_d = &*CONSONANT_D;
            for (i, row) in consonant_d.iter().enumerate() {
                if self.index < row.len() {
                    continue;
                }
                is_correct = self.check_correct_vowel(consonant_d, i, data);
                if !is_correct && self.index >= 2 && self.chr(self.index - 1) == KEY_D && is_consonant(self.chr(self.index - 2)) {
                    is_correct = true;
                }
                if is_correct {
                    self.insert_d();
                    return;
                }
            }
            self.insert_key(data, is_caps, true);
            return;
        }

        // If Mark key (Telex s, f, r, x, j; VNI 1, 2, 3, 4, 5)
        if self.is_mark_key(data) {
            for (_vowel_key, charset) in VOWEL_FOR_MARK.iter() {
                for (l, row) in charset.iter().enumerate() {
                    if self.index < row.len() {
                        continue;
                    }
                    if self.check_correct_vowel(charset, l, data) {
                        if self.is_key_s(data) {
                            self.insert_mark(MARK1_MASK, true);
                        } else if self.is_key_f(data) {
                            self.insert_mark(MARK2_MASK, true);
                        } else if self.is_key_r(data) {
                            self.insert_mark(MARK3_MASK, true);
                        } else if self.is_key_x(data) {
                            self.insert_mark(MARK4_MASK, true);
                        } else if self.is_key_j(data) {
                            self.insert_mark(MARK5_MASK, true);
                        }
                        return;
                    }
                }
            }
            self.insert_key(data, is_caps, true);
            return;
        }

        // Check vowel tone modifications (^ and w)
        let mut vei = 0;
        if self.input_type == InputType::Vni {
            for i in (0..self.index).rev() {
                let c = self.chr(i);
                if c == KEY_O || c == KEY_A || c == KEY_E {
                    vei = i;
                    break;
                }
            }
        }

        let key_for_aeo = if self.input_type != InputType::Vni {
            data
        } else {
            if data == KEY_7 || data == KEY_8 {
                KEY_W
            } else if data == KEY_6 {
                self.chr(vei)
            } else {
                data
            }
        };

        if let Some(charset) = VOWEL.get(&key_for_aeo) {
            let mut is_changed = false;
            for (i, row) in charset.iter().enumerate() {
                if self.index < row.len() {
                    continue;
                }
                if self.check_correct_vowel(charset, i, data) {
                    is_changed = true;
                    if self.is_key_double(data) {
                        self.insert_aoe(key_for_aeo);
                    } else if self.is_key_w(data) {
                        if self.input_type == InputType::Vni {
                            let mut v_idx = 0;
                            for j in (0..self.index).rev() {
                                let c = self.chr(j);
                                if c == KEY_O || c == KEY_U || c == KEY_A || c == KEY_E {
                                    v_idx = j;
                                    break;
                                }
                            }
                            if (data == KEY_7 && self.chr(v_idx) == KEY_A && (v_idx >= 1 && self.chr(v_idx - 1) != KEY_U))
                                || (data == KEY_8 && (self.chr(v_idx) == KEY_O || self.chr(v_idx) == KEY_U)) {
                                is_changed = false;
                                break;
                            }
                        }
                        self.insert_w();
                    }
                    break;
                }
            }
            if is_changed {
                return;
            }
        }

        if data == KEY_W && self.input_type != InputType::SimpleTelex1 {
            self.check_for_standalone_char(data, is_caps, KEY_U as u32);
        } else {
            self.insert_key(data, is_caps, true);
        }
    }

    #[inline]
    fn is_special_key(&self, key_code: u16) -> bool {
        match self.input_type {
            InputType::Telex => matches!(
                key_code,
                KEY_W | KEY_E | KEY_R | KEY_O | KEY_LEFT_BRACKET | KEY_RIGHT_BRACKET |
                KEY_A | KEY_S | KEY_D | KEY_F | KEY_J | KEY_Z | KEY_X
            ),
            InputType::Vni => matches!(
                key_code,
                KEY_1 | KEY_2 | KEY_3 | KEY_4 | KEY_5 | KEY_6 | KEY_7 | KEY_8 | KEY_9 | KEY_0
            ),
            InputType::SimpleTelex1 | InputType::SimpleTelex2 => matches!(
                key_code,
                KEY_W | KEY_E | KEY_R | KEY_O | KEY_A | KEY_S | KEY_D | KEY_F | KEY_J | KEY_Z | KEY_X
            ),
        }
    }


    #[inline]
    fn is_quick_telex_key(&self, code: u16) -> bool {
        self.index > 0
            && matches!(code, KEY_C | KEY_G | KEY_K | KEY_N | KEY_Q | KEY_P | KEY_T)
            && self.chr(self.index - 1) == code
    }

    pub fn check_macro(&mut self) -> bool {

        if !self.use_macro || self.has_handled_macro || self.macro_key.is_empty() {
            return false;
        }

        let mut word = String::new();
        for &k in &self.macro_key {
            let is_caps = (k & CAPS_MASK) != 0;
            let vk = (k & !CAPS_MASK) as u16;
            let ch = key_code_to_character(vk as u32);
            if ch != 0 {

                let mut c = ch as u8 as char;
                if is_caps {
                    c = c.to_ascii_uppercase();
                } else {
                    c = c.to_ascii_lowercase();
                }
                word.push(c);
            } else {
                return false;
            }
        }

        if word.is_empty() {
            return false;
        }

        if let Ok(table) = self.macro_table.lock() {
            if let Some(replacement) = table.lookup(&word, self.auto_caps_macro) {
                let macro_codes = crate::macro_engine::string_to_macro_key_codes(&replacement, self.code_table);
                self.hook_state.code = HookCodeState::ReplaceMacro;
                self.hook_state.backspace_count = self.macro_key.len() as u8;
                self.hook_state.macro_data = macro_codes;
                self.has_handled_macro = true;
                self.macro_key.clear();
                self.index = 0;
                return true;
            }
        }

        false
    }

    pub fn handle_english_mode(
        &mut self,
        data: u16,
        is_caps: bool,
        other_control_key: bool,
    ) {
        self.hook_state.code = HookCodeState::DoNothing;
        if other_control_key {
            self.macro_key.clear();
        } else if data == KEY_SPACE || Self::is_macro_break_code(data) {
            if self.use_macro && !self.has_handled_macro && self.check_macro() {
                // macro replaced
            } else {
                self.macro_key.clear();
            }
        } else if data == KEY_DELETE {
            self.macro_key.pop();
        } else if self.is_word_break(KeyEvent::Keyboard, data) && !Self::is_char_key_code(data) {
            self.macro_key.clear();
        } else {
            self.macro_key.push((data as u32) | if is_caps { CAPS_MASK } else { 0 });
        }
    }

    pub fn handle_event(

        &mut self,
        event: KeyEvent,
        state: KeyEventState,
        data: u16,
        caps_status: u8,
        other_control_key: bool,
    ) -> &HookState {
        let is_caps = caps_status == 1 || caps_status == 2;

        if (is_number_key(data) && caps_status == 1)
            || other_control_key
            || self.is_word_break(event, data)
            || (self.index == 0 && is_number_key(data)) {
            self.hook_state.code = HookCodeState::DoNothing;
            self.hook_state.backspace_count = 0;
            self.hook_state.new_char_count = 0;
            self.hook_state.ext_code = 1; // Word break

            if self.use_macro && Self::is_macro_break_code(data) && !self.has_handled_macro && self.check_macro() {
                // Macro matched on punctuation break
            } else if (self.quick_start_consonant || self.quick_end_consonant) && !self.temp_disable_key && Self::is_macro_break_code(data) {
                self.check_quick_consonant();
            } else if self.restore_if_wrong_spelling && self.is_word_break(event, data) {
                if !self.temp_disable_key && self.check_spelling {
                    self.check_spelling_internal(true);
                }
                if self.temp_disable_key && !self.check_restore_if_wrong_spelling(HookCodeState::RestoreAndStartNewSession) {
                    self.hook_state.code = HookCodeState::DoNothing;
                }
            }

            let is_char_key = state == KeyEventState::KeyDown && Self::is_char_key_code(data);
            if !is_char_key {
                self.special_char.clear();
                self.typing_states.clear();
            } else {
                if self.space_count > 0 {
                    self.save_word_repetition(KEY_SPACE as u32, self.space_count);
                    self.space_count = 0;
                } else {
                    self.save_word();
                }
                self.special_char.push((data as u32) | if is_caps { CAPS_MASK } else { 0 });
                self.hook_state.ext_code = 3;
            }

            if self.hook_state.code == HookCodeState::DoNothing {
                self.start_new_session();
                self.check_spelling = self.use_spell_checking_before;
                self.will_temp_off_engine = false;
            } else if self.hook_state.code == HookCodeState::ReplaceMacro || self.has_handle_quick_consonant {
                self.index = 0;
            }

            if self.upper_case_first_char {
                if data == KEY_DOT {
                    self.upper_case_status = 1;
                } else if data == KEY_ENTER || data == KEY_RETURN {
                    self.upper_case_status = 2;
                } else {
                    self.upper_case_status = 0;
                }
            }
        } else if data == KEY_SPACE {
            if self.use_macro && !self.has_handled_macro && self.check_macro() {
                self.space_count += 1;
            } else {
                if !self.temp_disable_key && self.check_spelling {
                    self.check_spelling_internal(true);
                }
                if (self.quick_start_consonant || self.quick_end_consonant) && !self.temp_disable_key && self.check_quick_consonant() {
                    self.space_count += 1;
                } else if self.restore_if_wrong_spelling && self.temp_disable_key && !self.has_handled_macro {
                    if !self.check_restore_if_wrong_spelling(HookCodeState::Restore) {
                        self.hook_state.code = HookCodeState::DoNothing;
                    }
                    self.space_count += 1;
                } else {
                    self.hook_state.code = HookCodeState::DoNothing;
                    self.space_count += 1;
                }
            }


            if self.upper_case_first_char && self.upper_case_status == 1 {
                self.upper_case_status = 2;
            }

            if self.space_count == 1 {
                if !self.special_char.is_empty() {
                    self.save_special_char();
                } else {
                    self.save_word();
                }
            }
            self.check_spelling = self.use_spell_checking_before;
            self.will_temp_off_engine = false;
        } else if data == KEY_DELETE {
            if !self.macro_key.is_empty() {
                self.macro_key.pop();
            }
            self.hook_state.code = HookCodeState::DoNothing;
            self.hook_state.ext_code = 2;


            if !self.special_char.is_empty() {
                self.special_char.pop();
                if self.special_char.is_empty() {
                    self.restore_last_typing_state();
                }
            } else if self.space_count > 0 {
                self.space_count -= 1;
                if self.space_count == 0 {
                    self.restore_last_typing_state();
                }
            } else {
                if self.state_index > 0 {
                    self.state_index -= 1;
                }
                if self.index > 0 {
                    self.index -= 1;
                    if !self.long_word_helper.is_empty() {
                        for i in (1..MAX_BUFF).rev() {
                            self.typing_word[i] = self.typing_word[i - 1];
                        }
                        if let Some(ch) = self.long_word_helper.pop() {
                            self.typing_word[0] = ch;
                            self.index += 1;
                        }
                    }
                    if self.check_spelling {
                        self.check_spelling_internal(false);
                    }
                }

                self.hook_state.backspace_count = 0;
                self.hook_state.new_char_count = 0;
                self.hook_state.ext_code = 2;

                if self.index == 0 {
                    self.start_new_session();
                    self.special_char.clear();
                    self.restore_last_typing_state();
                } else {
                    self.check_grammar(1);
                }
            }
        } else {
            // Main character keystroke
            if self.will_temp_off_engine {
                self.hook_state.code = HookCodeState::DoNothing;
                self.hook_state.ext_code = 3;
                return &self.hook_state;
            }

            if self.space_count > 0 {
                self.hook_state.backspace_count = 0;
                self.hook_state.new_char_count = 0;
                self.hook_state.ext_code = 0;
                self.start_new_session();
                self.save_word_repetition(KEY_SPACE as u32, self.space_count);
                self.space_count = 0;
            } else if !self.special_char.is_empty() {
                self.save_special_char();
            }

            self.insert_state(data, is_caps);
            self.macro_key.push((data as u32) | if is_caps { CAPS_MASK } else { 0 });


            if !self.is_special_key(data) || self.temp_disable_key {
                if self.quick_telex && self.is_quick_telex_key(data) {
                    self.handle_quick_telex(data, is_caps);
                    return &self.hook_state;
                } else {
                    self.hook_state.code = HookCodeState::DoNothing;
                    self.hook_state.backspace_count = 0;
                    self.hook_state.new_char_count = 0;
                    self.hook_state.ext_code = 3;
                    self.insert_key(data, is_caps, true);
                }
            } else {
                self.hook_state.code = HookCodeState::DoNothing;
                self.hook_state.ext_code = 3;
                self.handle_main_key(data, is_caps);
            }

            if !self.free_mark && data != KEY_D {
                if self.hook_state.code == HookCodeState::DoNothing {
                    self.check_grammar(-1);
                } else {
                    self.check_grammar(0);
                }
            }

            if self.hook_state.code == HookCodeState::Restore {
                self.insert_key(data, is_caps, true);
                if self.state_index > 0 {
                    self.state_index -= 1;
                }
            }

            if self.upper_case_first_char {
                if self.index == 1 && self.upper_case_status == 2 {
                    self.upper_case_first_character();
                }
                self.upper_case_status = 0;
            }

            // Case [ and ]
            let is_bracket = data == KEY_LEFT_BRACKET || data == KEY_RIGHT_BRACKET;
            if is_bracket {
                let first_char = self.hook_state.char_data[0] as u16;
                let is_first_bracket = first_char == KEY_LEFT_BRACKET || first_char == KEY_RIGHT_BRACKET;
                if is_first_bracket || self.input_type == InputType::SimpleTelex1 || self.input_type == InputType::SimpleTelex2 {
                    let bpc = if self.hook_state.code == HookCodeState::WillProcess {
                        self.hook_state.backspace_count as usize
                    } else {
                        0
                    };
                    if self.index.saturating_sub(bpc) > 0 {
                        self.index -= 1;
                        self.save_word();
                    }
                    self.index = 0;
                    self.temp_disable_key = false;
                    self.state_index = 0;
                    self.hook_state.ext_code = 3;
                    self.special_char.push((data as u32) | if is_caps { CAPS_MASK } else { 0 });
                }
            }
        }

        &self.hook_state
    }

    pub fn temp_off_spell_checking(&mut self) {
        if self.use_spell_checking_before {
            self.check_spelling = !self.check_spelling;
        }
    }

    pub fn set_check_spelling(&mut self, val: bool) {
        self.check_spelling = val;
        self.use_spell_checking_before = val;
    }

    pub fn temp_off_engine(&mut self, off: bool) {
        self.will_temp_off_engine = off;
    }

    /// Helper for simulating string typing and returning the resulting output string
    pub fn type_string(&mut self, text: &str) -> String {
        let mut output = String::new();
        for ch in text.chars() {
            let (key_code, is_caps) = match ch {
                'a'..='z' => ((ch.to_ascii_uppercase() as u8) as u16, false),
                'A'..='Z' => ((ch as u8) as u16, true),
                '0'..='9' => ((ch as u8) as u16, false),
                ' ' => (KEY_SPACE, false),
                '\n' | '\r' => (KEY_ENTER, false),
                '\t' => (KEY_TAB, false),
                '\x08' => (KEY_DELETE, false),
                '[' => (KEY_LEFT_BRACKET, false),
                '{' => (KEY_LEFT_BRACKET, true),
                ']' => (KEY_RIGHT_BRACKET, false),
                '}' => (KEY_RIGHT_BRACKET, true),
                '.' => (KEY_DOT, false),
                '>' => (KEY_DOT, true),
                ',' => (KEY_COMMA, false),
                '<' => (KEY_COMMA, true),
                ';' => (KEY_SEMICOLON, false),
                ':' => (KEY_SEMICOLON, true),
                '\'' => (KEY_QUOTE, false),
                '"' => (KEY_QUOTE, true),
                '/' => (KEY_SLASH, false),
                '?' => (KEY_SLASH, true),
                '-' => (KEY_MINUS, false),
                '_' => (KEY_MINUS, true),
                '=' => (KEY_EQUALS, false),
                '+' => (KEY_EQUALS, true),
                '`' => (KEY_BACKQUOTE, false),
                '~' => (KEY_BACKQUOTE, true),
                _ => (ch as u16, ch.is_ascii_uppercase()),
            };

            let state = self.handle_event(
                KeyEvent::Keyboard,
                KeyEventState::KeyDown,
                key_code,
                if is_caps { 1 } else { 0 },
                false,
            );

            match state.code {
                HookCodeState::DoNothing => {
                    output.push(ch);
                }
                HookCodeState::WillProcess | HookCodeState::Restore | HookCodeState::RestoreAndStartNewSession => {
                    for _ in 0..state.backspace_count {
                        output.pop();
                    }
                    for i in (0..state.new_char_count as usize).rev() {
                        let c_data = state.char_data[i];
                        let c = if (c_data & CHAR_CODE_MASK) != 0 {
                            char::from_u32(c_data & 0xFFFF).unwrap_or('?')
                        } else {
                            let k = c_data & CHAR_MASK;
                            let caps = (c_data & CAPS_MASK) != 0;
                            let ch_out = key_code_to_character(k | if caps { CAPS_MASK } else { 0 });
                            char::from_u32(ch_out as u32).unwrap_or('?')
                        };
                        output.push(c);
                    }
                    if state.code == HookCodeState::Restore || state.code == HookCodeState::RestoreAndStartNewSession {
                        let k = key_code as u32 | if is_caps { CAPS_MASK } else { 0 };
                        let ch_out = key_code_to_character(k);
                        if ch_out != 0 {
                            if let Some(c) = char::from_u32(ch_out as u32) {
                                output.push(c);
                            }
                        }
                    }
                }
                HookCodeState::BreakWord => {
                    output.push(ch);
                }
                HookCodeState::ReplaceMacro => {
                    for _ in 0..state.backspace_count {
                        output.pop();
                    }
                    for &m_code in state.macro_data.iter() {
                        let c = if (m_code & CHAR_CODE_MASK) != 0 {
                            char::from_u32(m_code & 0xFFFF).unwrap_or('?')
                        } else {
                            let ch_out = key_code_to_character(m_code);
                            char::from_u32(ch_out as u32).unwrap_or('?')
                        };
                        output.push(c);
                    }
                    output.push(ch);
                }
            }
        }
        output
    }
}
