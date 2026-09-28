// DataType & Constants port of OpenKey to Rust

pub const MAX_BUFF: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEvent {
    Keyboard,
    Mouse,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyEventState {
    KeyDown,
    KeyUp,
    MouseDown,
    MouseUp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InputType {
    #[default]
    Telex = 0,
    Vni = 1,
    SimpleTelex1 = 2,
    SimpleTelex2 = 3,
}

impl InputType {
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => InputType::Telex,
            1 => InputType::Vni,
            2 => InputType::SimpleTelex1,
            3 => InputType::SimpleTelex2,
            _ => InputType::Telex,
        }
    }

    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodeTable {
    #[default]
    Unicode = 0,
    Tcvn3 = 1,
    VniWindows = 2,
    UnicodeCompound = 3,
    Cp1258 = 4,
}

impl CodeTable {
    pub fn from_u32(val: u32) -> Self {
        match val {
            0 => CodeTable::Unicode,
            1 => CodeTable::Tcvn3,
            2 => CodeTable::VniWindows,
            3 => CodeTable::UnicodeCompound,
            4 => CodeTable::Cp1258,
            _ => CodeTable::Unicode,
        }
    }

    pub fn to_u32(&self) -> u32 {
        *self as u32
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HookCodeState {
    #[default]
    DoNothing = 0,
    WillProcess = 1,
    BreakWord = 2,
    Restore = 3,
    ReplaceMacro = 4,
    RestoreAndStartNewSession = 5,
}

#[derive(Debug, Clone)]
pub struct HookState {
    pub code: HookCodeState,
    pub backspace_count: u8,
    pub new_char_count: u8,
    pub ext_code: u8,
    pub char_data: [u32; MAX_BUFF],
    pub macro_key: Vec<u32>,
    pub macro_data: Vec<u32>,
}

impl Default for HookState {
    fn default() -> Self {
        Self {
            code: HookCodeState::DoNothing,
            backspace_count: 0,
            new_char_count: 0,
            ext_code: 0,
            char_data: [0; MAX_BUFF],
            macro_key: Vec::new(),
            macro_data: Vec::new(),
        }
    }
}

// Internal engine data bitmasks
pub const CAPS_MASK: u32 = 0x10000;
pub const TONE_MASK: u32 = 0x20000;
pub const TONEW_MASK: u32 = 0x40000;

pub const MARK1_MASK: u32 = 0x80000;  // Dấu Sắc - á
pub const MARK2_MASK: u32 = 0x100000; // Dấu Huyền - à
pub const MARK3_MASK: u32 = 0x200000; // Dấu Hỏi - ả
pub const MARK4_MASK: u32 = 0x400000; // Dấu Ngã - ã
pub const MARK5_MASK: u32 = 0x800000; // Dấu Nặng - ạ

pub const MARK_MASK: u32 = 0xF80000;
pub const CHAR_MASK: u32 = 0xFFFF;
pub const STANDALONE_MASK: u32 = 0x1000000;
pub const CHAR_CODE_MASK: u32 = 0x2000000;
pub const PURE_CHARACTER_MASK: u32 = 0x80000000;

pub const END_CONSONANT_MASK: u16 = 0x4000;
pub const CONSONANT_ALLOW_MASK: u16 = 0x8000;

// Key code constants matching Win32 virtual keys
pub const KEY_ESC: u16 = 0x1B;
pub const KEY_DELETE: u16 = 0x08; // VK_BACK
pub const KEY_TAB: u16 = 0x09;
pub const KEY_ENTER: u16 = 0x0D;
pub const KEY_RETURN: u16 = 0x0D;
pub const KEY_SPACE: u16 = 0x20;
pub const KEY_LEFT: u16 = 0x25;
pub const KEY_RIGHT: u16 = 0x27;
pub const KEY_DOWN: u16 = 0x28;
pub const KEY_UP: u16 = 0x26;

pub const KEY_EMPTY: u16 = 256;
pub const KEY_A: u16 = 0x41;
pub const KEY_B: u16 = 0x42;
pub const KEY_C: u16 = 0x43;
pub const KEY_D: u16 = 0x44;
pub const KEY_E: u16 = 0x45;
pub const KEY_F: u16 = 0x46;
pub const KEY_G: u16 = 0x47;
pub const KEY_H: u16 = 0x48;
pub const KEY_I: u16 = 0x49;
pub const KEY_J: u16 = 0x4A;
pub const KEY_K: u16 = 0x4B;
pub const KEY_L: u16 = 0x4C;
pub const KEY_M: u16 = 0x4D;
pub const KEY_N: u16 = 0x4E;
pub const KEY_O: u16 = 0x4F;
pub const KEY_P: u16 = 0x50;
pub const KEY_Q: u16 = 0x51;
pub const KEY_R: u16 = 0x52;
pub const KEY_S: u16 = 0x53;
pub const KEY_T: u16 = 0x54;
pub const KEY_U: u16 = 0x55;
pub const KEY_V: u16 = 0x56;
pub const KEY_W: u16 = 0x57;
pub const KEY_X: u16 = 0x58;
pub const KEY_Y: u16 = 0x59;
pub const KEY_Z: u16 = 0x5A;

pub const KEY_1: u16 = 0x31;
pub const KEY_2: u16 = 0x32;
pub const KEY_3: u16 = 0x33;
pub const KEY_4: u16 = 0x34;
pub const KEY_5: u16 = 0x35;
pub const KEY_6: u16 = 0x36;
pub const KEY_7: u16 = 0x37;
pub const KEY_8: u16 = 0x38;
pub const KEY_9: u16 = 0x39;
pub const KEY_0: u16 = 0x30;

pub const KEY_LEFT_BRACKET: u16 = 0xDB;  // [ {
pub const KEY_RIGHT_BRACKET: u16 = 0xDD; // ] }

pub const KEY_LEFT_SHIFT: u16 = 160;
pub const KEY_RIGHT_SHIFT: u16 = 161;
pub const KEY_DOT: u16 = 0xBE;

pub const KEY_BACKQUOTE: u16 = 220;
pub const KEY_MINUS: u16 = 189;
pub const KEY_EQUALS: u16 = 187;
pub const KEY_BACK_SLASH: u16 = 222;
pub const KEY_SEMICOLON: u16 = 186;
pub const KEY_QUOTE: u16 = 192;
pub const KEY_COMMA: u16 = 0xBC;
pub const KEY_SLASH: u16 = 191;

// Windows virtual keys for break code
pub const VK_INSERT: u16 = 0x2D;
pub const VK_HOME: u16 = 0x24;
pub const VK_END: u16 = 0x23;
pub const VK_DELETE_SYS: u16 = 0x2E;
pub const VK_PRIOR: u16 = 0x21; // Page Up
pub const VK_NEXT: u16 = 0x22;  // Page Down
pub const VK_SNAPSHOT: u16 = 0x2C;
pub const VK_PRINT: u16 = 0x2A;
pub const VK_SELECT: u16 = 0x29;
pub const VK_HELP: u16 = 0x2F;
pub const VK_EXECUTE: u16 = 0x2B;
pub const VK_NUMLOCK: u16 = 0x90;
pub const VK_SCROLL: u16 = 0x91;

#[inline]
pub fn is_consonant(key_code: u16) -> bool {
    !matches!(key_code, KEY_A | KEY_E | KEY_U | KEY_Y | KEY_I | KEY_O)
}

#[inline]
pub fn is_double_code(code: usize) -> bool {
    code == 2 || code == 3
}

#[inline]
pub fn is_vni_code(code: usize) -> bool {
    code == 2
}

#[inline]
pub fn is_number_key(code: u16) -> bool {
    matches!(code, KEY_1 | KEY_2 | KEY_3 | KEY_4 | KEY_5 | KEY_6 | KEY_7 | KEY_8 | KEY_9 | KEY_0)
}
