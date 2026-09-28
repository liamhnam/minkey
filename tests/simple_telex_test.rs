use minkey::{InputType, VietnameseEngine};

#[test]
fn test_simple_telex_rules() {
    // Simple Telex 1:
    // w does NOT produce standalone ư
    let mut st1 = VietnameseEngine::new();
    st1.input_type = InputType::SimpleTelex1;
    assert_eq!(st1.type_string("w"), "w");
    // But modifier w still works: uw -> ư, ow -> ơ, aw -> ă
    let mut st1_mod = VietnameseEngine::new();
    st1_mod.input_type = InputType::SimpleTelex1;
    assert_eq!(st1_mod.type_string("uw"), "ư");

    // Simple Telex 2:
    // w DOES produce standalone ư
    let mut st2 = VietnameseEngine::new();
    st2.input_type = InputType::SimpleTelex2;
    assert_eq!(st2.type_string("w"), "ư");

    // In both Simple Telex 1 and 2: [ and ] are NOT turned into ơ and ư
    let mut st1_bracket = VietnameseEngine::new();
    st1_bracket.input_type = InputType::SimpleTelex1;
    assert_eq!(st1_bracket.type_string("["), "[");
    assert_eq!(st1_bracket.type_string("]"), "]");

    let mut st2_bracket = VietnameseEngine::new();
    st2_bracket.input_type = InputType::SimpleTelex2;
    assert_eq!(st2_bracket.type_string("["), "[");
    assert_eq!(st2_bracket.type_string("]"), "]");
}
