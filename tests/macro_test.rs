use minkey::macro_engine::*;
use minkey::VietnameseEngine;

#[test]
fn test_macro_basic() {
    let mut table = MacroTable::new();
    table.add("ms", "millisecond");
    table.add("ko", "không");
    table.add("dc", "được");

    assert_eq!(table.len(), 3);
    assert!(table.has("ms"));
    assert!(table.has("ko"));
    assert!(!table.has("xyz"));

    assert_eq!(table.lookup("ms", false), Some("millisecond".to_string()));
    assert_eq!(table.lookup("ko", false), Some("không".to_string()));

    table.delete("ms");
    assert_eq!(table.len(), 2);
    assert!(!table.has("ms"));
}

#[test]
fn test_macro_autocaps() {
    let mut table = MacroTable::new();
    table.add("ko", "không");
    table.add("dc", "được");
    table.add("vn", "Việt Nam");

    // All lowercase
    assert_eq!(table.lookup("ko", true), Some("không".to_string()));

    // Title case
    assert_eq!(table.lookup("Ko", true), Some("Không".to_string()));
    assert_eq!(table.lookup("Dc", true), Some("Được".to_string()));

    // ALL CAPS
    assert_eq!(table.lookup("KO", true), Some("KHÔNG".to_string()));
    assert_eq!(table.lookup("DC", true), Some("ĐƯỢC".to_string()));
}

#[test]
fn test_macro_binary_registry_format() {
    let mut table = MacroTable::new();
    table.add("ko", "không");
    table.add("dc", "được");
    table.add("test", "thử nghiệm gõ tắt 123");

    let bytes = table.to_binary();
    assert!(bytes.len() > 2);

    let restored = MacroTable::from_binary(&bytes);
    assert_eq!(restored.len(), 3);
    assert_eq!(restored.lookup("ko", false), Some("không".to_string()));
    assert_eq!(restored.lookup("dc", false), Some("được".to_string()));
    assert_eq!(restored.lookup("test", false), Some("thử nghiệm gõ tắt 123".to_string()));
}

#[test]
fn test_macro_unikey_txt_import_export() {
    let mut table = MacroTable::new();
    table.add("ko", "không");
    table.add("dc", "được");

    let exported = table.export_txt();
    assert!(exported.starts_with(";Compatible OpenKey Macro Data file for UniKey*** version=1 ***"));
    assert!(exported.contains("ko:không"));
    assert!(exported.contains("dc:được"));

    let mut imported = MacroTable::new();
    imported.import_txt(&exported, false);
    assert_eq!(imported.len(), 2);
    assert_eq!(imported.lookup("ko", false), Some("không".to_string()));
    assert_eq!(imported.lookup("dc", false), Some("được".to_string()));
}

#[test]
fn test_engine_typing_macro_expansion() {
    let mut engine = VietnameseEngine::new();
    engine.use_macro = true;
    engine.auto_caps_macro = true;

    {
        let mut table = engine.macro_table.lock().unwrap();
        table.add("ko", "không");
        table.add("dc", "được");
    }

    // Typing lowercase macro + space: "ko " -> "không "
    let res1 = engine.type_string("ko ");
    assert_eq!(res1, "không ");

    // Typing titlecase macro + space: "Ko " -> "Không "
    let res2 = engine.type_string("Ko ");
    assert_eq!(res2, "Không ");

    // Typing uppercase macro + space: "KO " -> "KHÔNG "
    let res3 = engine.type_string("KO ");
    assert_eq!(res3, "KHÔNG ");

    // Typing "dc " -> "được "
    let res4 = engine.type_string("dc ");
    assert_eq!(res4, "được ");
}

#[test]
fn test_macro_erases_shown_letters_not_typed_keys() {
    // "aa" is typed with 2 keys but shows as the single letter "â": expanding it must
    // erase 1 character, not eat the space before it
    let mut engine = VietnameseEngine::new();
    engine.use_macro = true;
    engine.macro_table.lock().unwrap().add("aa", "AA-macro");
    assert_eq!(engine.type_string("x aa "), "x AA-macro ");
}

#[test]
fn test_macro_keeps_non_vietnamese_unicode() {
    let mut engine = VietnameseEngine::new();
    engine.use_macro = true;
    engine.macro_table.lock().unwrap().add("tm", "™ → “ok” 😀");
    assert_eq!(engine.type_string("tm "), "™ → “ok” 😀 ");

    // A code point above U+FFFF must not be truncated into a Vietnamese letter (U+11EA1 -> U+1EA1 "ạ")
    let codes = string_to_macro_key_codes("\u{11EA1}", 0);
    assert_eq!(codes.len(), 1);
    assert_eq!(minkey::tables::pure_character(codes[0]), Some('\u{11EA1}'));
}
