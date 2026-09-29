use minkey::config::{DEFAULT_SWITCH_STATUS, EMPTY_HOTKEY};
use minkey::settings::{Hotkey, decode_hotkey, encode_hotkey};

#[test]
fn test_default_switch_key_is_alt_z() {
    let hk = decode_hotkey(DEFAULT_SWITCH_STATUS);
    assert!(hk.alt && !hk.ctrl && !hk.win && !hk.shift);
    assert_eq!(hk.key, Some('Z'));
    assert_eq!(encode_hotkey(&hk), DEFAULT_SWITCH_STATUS);
}

#[test]
fn test_modifier_only_hotkey() {
    // Ctrl + Shift with no key: low byte 0xFE, which the hook treats as "modifiers only"
    let code = encode_hotkey(&Hotkey { ctrl: true, shift: true, ..Default::default() });
    assert_eq!(code & 0xFF, 0xFE);
    let hk = decode_hotkey(code);
    assert!(hk.ctrl && hk.shift && hk.key.is_none());
}

#[test]
fn test_no_key_and_no_modifier_means_no_hotkey() {
    // Clearing the key box must disable the hotkey, not bind a bare letter
    assert_eq!(encode_hotkey(&Hotkey::default()), EMPTY_HOTKEY);
    assert_eq!(encode_hotkey(&Hotkey { beep: true, ..Default::default() }), EMPTY_HOTKEY);
    assert_eq!(decode_hotkey(EMPTY_HOTKEY).key, None);
}

#[test]
fn test_only_letters_and_digits_are_keys() {
    // Punctuation has a different virtual-key code than its ASCII code
    let code = encode_hotkey(&Hotkey { ctrl: true, key: Some(','), ..Default::default() });
    assert_eq!(decode_hotkey(code).key, None);
    let code = encode_hotkey(&Hotkey { ctrl: true, key: Some('k'), beep: true, ..Default::default() });
    let hk = decode_hotkey(code);
    assert_eq!(hk.key, Some('K'));
    assert!(hk.beep);
}
