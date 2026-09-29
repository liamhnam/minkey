use minkey::VietnameseEngine;

#[test]
fn test_tieengs_vieetj() {
    let mut engine = VietnameseEngine::new();
    let res = engine.type_string("tieengs vieetj");
    assert_eq!(res, "tiếng việt");
}

#[test]
fn test_toanf_duwowngj() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("toanf"), "toàn");

    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("duwowngj"), "dượng");
}

#[test]
fn test_dd_to_dbar() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("dd"), "đ");

    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("DD"), "Đ");
}

#[test]
fn test_brackets_to_vowels() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("["), "ơ");

    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("]"), "ư");

    let mut engine3 = VietnameseEngine::new();
    assert_eq!(engine3.type_string("{"), "Ơ");

    let mut engine4 = VietnameseEngine::new();
    assert_eq!(engine4.type_string("}"), "Ư");
}

#[test]
fn test_modern_vs_classic_mark() {
    // Classic orthography (default: use_modern_orthography = false): hòa, thùy
    let mut engine_classic = VietnameseEngine::new();
    engine_classic.use_modern_orthography = false;
    assert_eq!(engine_classic.type_string("hoaf"), "hòa");
    let mut engine_classic2 = VietnameseEngine::new();
    engine_classic2.use_modern_orthography = false;
    assert_eq!(engine_classic2.type_string("thuyf"), "thùy");

    // Modern orthography (use_modern_orthography = true): hoà, thuỳ
    let mut engine_modern = VietnameseEngine::new();
    engine_modern.use_modern_orthography = true;
    assert_eq!(engine_modern.type_string("hoaf"), "hoà");
    let mut engine_modern2 = VietnameseEngine::new();
    engine_modern2.use_modern_orthography = true;
    assert_eq!(engine_modern2.type_string("thuyf"), "thuỳ");
}

#[test]
fn test_unmark_with_z() {
    let mut engine = VietnameseEngine::new();
    // xoas -> xóa -> xoaz -> xoa
    assert_eq!(engine.type_string("xoasz"), "xoa");

    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("toanfz"), "toan");
}

#[test]
fn test_caps_preservation() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("TIEENGS"), "TIẾNG");

    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("Toanf"), "Toàn");
}

#[test]
fn test_sentence_typing() {
    let mut engine = VietnameseEngine::new();
    let text = "Tooi laf nguowfi Vieetj Nam.";
    assert_eq!(engine.type_string(text), "Tôi là người Việt Nam.");
}

#[test]
fn test_all_tones() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("mas"), "má");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("maf"), "mà");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("mar"), "mả");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("max"), "mã");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("maj"), "mạ");
}

#[test]
fn test_all_circumflex_and_horn() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("aa"), "â");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("ee"), "ê");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("oo"), "ô");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("aw"), "ă");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("ow"), "ơ");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("uw"), "ư");
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("w"), "ư");
}

#[test]
fn test_duplicate_mark_undo() {
    let mut engine = VietnameseEngine::new();
    // In Telex, typing the same tone mark again removes the mark and appends the key
    // toans -> toán, toanss -> toans
    assert_eq!(engine.type_string("toans"), "toán");
    let mut engine2 = VietnameseEngine::new();
    assert_eq!(engine2.type_string("toanss"), "toans");
}

#[test]
fn test_restore_wrong_spelling_on_space() {
    let mut engine = VietnameseEngine::new();
    engine.check_spelling = true;
    engine.restore_if_wrong_spelling = true;

    // Type a word with tone mark then an invalid letter for Vietnamese grammar, e.g. "toanf" -> "toàn", then "k" -> "toànk" (not valid), space restores "toanfk "
    let res = engine.type_string("toanfk ");
    println!("res for 'toanfk ': '{}'", res);
    assert_eq!(res, "toanfk ");
}






// Regressions found by tests/openkey_parity_test.rs

#[test]
fn test_gi_takes_tone() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("gif gis gix gij gir"), "gì gí gĩ gị gỉ");
}

#[test]
fn test_w_that_does_not_apply_is_typed_as_letter() {
    let mut engine = VietnameseEngine::new();
    engine.set_check_spelling(false);
    engine.restore_if_wrong_spelling = false;
    // "ao" can't take a horn: "w" must be a normal letter, not duplicate the word
    assert_eq!(engine.type_string("taorw"), "tảow");
}

#[test]
fn test_capitalised_thuo() {
    let mut engine = VietnameseEngine::new();
    assert_eq!(engine.type_string("Thuowr"), "Thuở");
}

#[test]
fn test_uo_w_without_final_is_u_o_horn() {
    // "uo" + w ending the word: only "o" gets the horn (huơ tay, khuơ), OpenKey gave "hươ"
    for (keys, want) in [("huow ", "huơ "), ("khuow ", "khuơ "), ("huowf ", "huờ "), ("quow ", "quơ ")] {
        assert_eq!(VietnameseEngine::new().type_string(keys), want, "keys {keys:?}");
    }
    // A final, i or u typed afterwards makes it "ươ", whatever the order of w and the tone
    for (keys, want) in [
        ("huowng ", "hương "), ("dduowcj ", "được "), ("nguowif ", "người "), ("huowu ", "hươu "),
        ("ruowuj ", "rượu "), ("buowus ", "bướu "), ("tuowrng ", "tưởng "), ("thuowr ", "thuở "),
        ("dduowngf ", "đường "), ("muowngj ", "mượng "),
    ] {
        assert_eq!(VietnameseEngine::new().type_string(keys), want, "keys {keys:?}");
    }
}

#[test]
fn test_second_w_undoes_u_o_horn() {
    // Double w undoes the horn for "uơ" just like for "ươ"
    assert_eq!(VietnameseEngine::new().type_string("huoww"), "huow");
    assert_eq!(VietnameseEngine::new().type_string("thuoww"), "thuow");
    assert_eq!(VietnameseEngine::new().type_string("huongww"), "huongw");
}
