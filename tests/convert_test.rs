use minkey::convert::{convert_util, ConvertOptions};

#[test]
fn test_unicode_to_all_code_tables_roundtrip() {
    let original = "Cộng hòa xã hội chủ nghĩa Việt Nam. Độc lập Tự do Hạnh phúc!";

    // 0 -> 1: Unicode to TCVN3
    let mut to_tcvn3 = ConvertOptions::default();
    to_tcvn3.from_code = 0;
    to_tcvn3.to_code = 1;
    let tcvn3_text = convert_util(original, &to_tcvn3);
    assert_ne!(tcvn3_text, original);

    // 1 -> 0: TCVN3 back to Unicode
    let mut from_tcvn3 = ConvertOptions::default();
    from_tcvn3.from_code = 1;
    from_tcvn3.to_code = 0;
    let roundtrip_tcvn3 = convert_util(&tcvn3_text, &from_tcvn3);
    assert_eq!(roundtrip_tcvn3, original);

    // 0 -> 2: Unicode to VNI Windows
    let mut to_vni = ConvertOptions::default();
    to_vni.from_code = 0;
    to_vni.to_code = 2;
    let vni_text = convert_util(original, &to_vni);
    assert_ne!(vni_text, original);

    // 2 -> 0: VNI Windows back to Unicode
    let mut from_vni = ConvertOptions::default();
    from_vni.from_code = 2;
    from_vni.to_code = 0;
    let roundtrip_vni = convert_util(&vni_text, &from_vni);
    assert_eq!(roundtrip_vni, original);

    // 0 -> 3: Unicode to Unicode Compound
    let mut to_compound = ConvertOptions::default();
    to_compound.from_code = 0;
    to_compound.to_code = 3;
    let compound_text = convert_util(original, &to_compound);
    assert_ne!(compound_text, original);

    // 3 -> 0: Unicode Compound back to Unicode
    let mut from_compound = ConvertOptions::default();
    from_compound.from_code = 3;
    from_compound.to_code = 0;
    let roundtrip_compound = convert_util(&compound_text, &from_compound);
    assert_eq!(roundtrip_compound, original);

    // 0 -> 4: Unicode to CP 1258
    let mut to_1258 = ConvertOptions::default();
    to_1258.from_code = 0;
    to_1258.to_code = 4;
    let cp1258_text = convert_util(original, &to_1258);
    assert_ne!(cp1258_text, original);

    // 4 -> 0: CP 1258 back to Unicode
    let mut from_1258 = ConvertOptions::default();
    from_1258.from_code = 4;
    from_1258.to_code = 0;
    let roundtrip_1258 = convert_util(&cp1258_text, &from_1258);
    assert_eq!(roundtrip_1258, original);
}

#[test]
fn test_remove_marks() {
    let original = "Cộng hòa xã hội chủ nghĩa Việt Nam, Độc lập - Tự do - Hạnh phúc.";
    let mut opts = ConvertOptions::default();
    opts.from_code = 0;
    opts.to_code = 0;
    opts.remove_mark = true;

    let result = convert_util(original, &opts);
    assert_eq!(result, "Cong hoa xa hoi chu nghia Viet Nam, Doc lap - Tu do - Hanh phuc.");
}

#[test]
fn test_casing_options() {
    let input = "tiếng việt nam";

    // All Caps
    let mut opts_caps = ConvertOptions::default();
    opts_caps.from_code = 0;
    opts_caps.to_code = 0;
    opts_caps.to_all_caps = true;
    assert_eq!(convert_util(input, &opts_caps), "TIẾNG VIỆT NAM");

    // All Non Caps
    let mut opts_non_caps = ConvertOptions::default();
    opts_non_caps.from_code = 0;
    opts_non_caps.to_code = 0;
    opts_non_caps.to_all_non_caps = true;
    assert_eq!(convert_util("TIẾNG VIỆT NAM", &opts_non_caps), "tiếng việt nam");

    // Title Case (Each Word)
    let mut opts_title = ConvertOptions::default();
    opts_title.from_code = 0;
    opts_title.to_code = 0;
    opts_title.to_caps_each_word = true;
    assert_eq!(convert_util(input, &opts_title), "Tiếng Việt Nam");

    // Sentence Case (First Letter)
    let mut opts_sentence = ConvertOptions::default();
    opts_sentence.from_code = 0;
    opts_sentence.to_code = 0;
    opts_sentence.to_caps_first_letter = true;
    assert_eq!(
        convert_util("tiếng việt. xin chào! bạn khỏe không?", &opts_sentence),
        "Tiếng việt. Xin chào! Bạn khỏe không?"
    );
}

#[test]
fn test_remove_mark_with_all_caps() {
    let input = "tiếng việt";
    let mut opts = ConvertOptions::default();
    opts.from_code = 0;
    opts.to_code = 0;
    opts.remove_mark = true;
    opts.to_all_caps = true;
    assert_eq!(convert_util(input, &opts), "TIENG VIET");
}
