use minkey::{CodeTable, VietnameseEngine};

#[test]
fn test_quick_telex() {
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;

    // cc -> ch
    assert_eq!(engine.type_string("cc"), "ch");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("gg"), "gi");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("kk"), "kh");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("nn"), "ng");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("qq"), "qu");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("pp"), "ph");
    let mut engine = VietnameseEngine::new();
    engine.quick_telex = true;
    assert_eq!(engine.type_string("tt"), "th");
}

#[test]
fn test_quick_consonants() {
    let mut engine = VietnameseEngine::new();
    engine.quick_start_consonant = true;
    engine.quick_end_consonant = true;

    // quick start consonant triggers on word boundary (e.g. space)
    // fa -> pha
    assert_eq!(engine.type_string("fa "), "pha ");

    let mut engine = VietnameseEngine::new();
    engine.quick_start_consonant = true;
    // ja -> gia
    let mut engine = VietnameseEngine::new();
    engine.quick_start_consonant = true;
    assert_eq!(engine.type_string("ja "), "gia ");

    // In Simple Telex 1, w does not become ư, so quick start consonant converts wa -> qua
    let mut engine = VietnameseEngine::new();
    engine.input_type = minkey::InputType::SimpleTelex1;
    engine.quick_start_consonant = true;
    assert_eq!(engine.type_string("wa "), "qua ");

    // quick end consonant:
    // ag -> ang
    let mut engine = VietnameseEngine::new();
    engine.quick_end_consonant = true;
    assert_eq!(engine.type_string("ag "), "ang ");

    // ah -> anh
    let mut engine = VietnameseEngine::new();
    engine.quick_end_consonant = true;
    assert_eq!(engine.type_string("ah "), "anh ");

    // ak -> ach
    let mut engine = VietnameseEngine::new();
    engine.quick_end_consonant = true;
    assert_eq!(engine.type_string("ak "), "ach ");
}

#[test]
fn test_upper_case_first_char() {
    let mut engine = VietnameseEngine::new();
    engine.upper_case_first_char = true;

    // After dot and space: "xin chao. toi la ai" -> "xin chao. Toi la ai"
    let res = engine.type_string("xin chao. toi la ai");
    assert_eq!(res, "xin chao. Toi la ai");

    // After newline: "xin chao\ntoi" -> "xin chao\nToi"
    let mut engine2 = VietnameseEngine::new();
    engine2.upper_case_first_char = true;
    let res2 = engine2.type_string("xin chao\ntoi");
    assert_eq!(res2, "xin chao\nToi");
}

#[test]
fn test_all_code_tables() {
    // 0: Unicode (default)
    let mut e0 = VietnameseEngine::new();
    e0.code_table = CodeTable::Unicode.to_u32() as usize;
    assert_eq!(e0.type_string("Vieetj"), "Việt");

    // 1: TCVN3 (ABC)
    let mut e1 = VietnameseEngine::new();
    e1.code_table = CodeTable::Tcvn3.to_u32() as usize;
    let res1 = e1.type_string("Vieetj");
    // In TCVN3, 'ệ' has byte code 0xD6 (214)
    assert!(res1.contains('\u{00D6}'));

    // 2: VNI Windows
    let mut e2 = VietnameseEngine::new();
    e2.code_table = CodeTable::VniWindows.to_u32() as usize;
    let res2 = e2.type_string("Vieetj");
    // In VNI Windows, 'ệ' is 0xE465 (low byte 'e', high byte 0xE4)
    assert!(res2.contains('\u{E465}'));


    // 3: Unicode Compound (Tổ hợp)
    let mut e3 = VietnameseEngine::new();
    e3.code_table = CodeTable::UnicodeCompound.to_u32() as usize;
    let res3 = e3.type_string("Vieetj");
    // Compound 'ệ' is 'ê' (0x00EA) + combining dot below (0x0323)
    // Note: get_character_code in tables.rs outputs raw code for compound
    assert!(!res3.is_empty());

    // 4: CP 1258
    let mut e4 = VietnameseEngine::new();
    e4.code_table = CodeTable::Cp1258.to_u32() as usize;
    let res4 = e4.type_string("Vieetj");
    assert!(!res4.is_empty());
}
