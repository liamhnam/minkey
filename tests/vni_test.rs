use minkey::{InputType, VietnameseEngine};

#[test]
fn test_vni_basic() {
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;

    // tie6ng1 vie6t5 -> tiếng việt
    assert_eq!(engine.type_string("tie6ng1 vie6t5"), "tiếng việt");

    // toan2 -> toàn
    let mut engine2 = VietnameseEngine::new();
    engine2.input_type = InputType::Vni;
    assert_eq!(engine2.type_string("toan2"), "toàn");

    // du7o7ng5 -> dượng
    let mut engine3 = VietnameseEngine::new();
    engine3.input_type = InputType::Vni;
    assert_eq!(engine3.type_string("du7o7ng5"), "dượng");

    // d9u7o7ng5 -> đượng
    let mut engine3b = VietnameseEngine::new();
    engine3b.input_type = InputType::Vni;
    assert_eq!(engine3b.type_string("d9u7o7ng5"), "đượng");


    // d9 -> đ, D9 -> Đ
    let mut engine4 = VietnameseEngine::new();
    engine4.input_type = InputType::Vni;
    assert_eq!(engine4.type_string("d9"), "đ");
    let mut engine5 = VietnameseEngine::new();
    engine5.input_type = InputType::Vni;
    assert_eq!(engine5.type_string("D9"), "Đ");
}

#[test]
fn test_vni_all_tones_and_marks() {
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;

    assert_eq!(engine.type_string("ma1"), "má");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("ma2"), "mà");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("ma3"), "mả");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("ma4"), "mã");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("ma5"), "mạ");

    // 6 for circumflex: a6->â, e6->ê, o6->ô
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("a6"), "â");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("e6"), "ê");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("o6"), "ô");

    // 7 for horn: o7->ơ, u7->ư
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("o7"), "ơ");
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("u7"), "ư");

    // 8 for breve: a8->ă
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("a8"), "ă");

    // 0 for unmark: ma10 -> ma
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    assert_eq!(engine.type_string("ma10"), "ma");
}

#[test]
fn test_vni_sentence() {
    let mut engine = VietnameseEngine::new();
    engine.input_type = InputType::Vni;
    let res = engine.type_string("To6i la2 ngu7o72i Vie6t5 Nam.");
    assert_eq!(res, "Tôi là người Việt Nam.");
}

#[test]
fn test_vni_dieu_with_u_final() {
    // "điu" was missing from the đ table: "diu9" stayed "diu9"
    let mut engine = VietnameseEngine::new();
    engine.input_type = minkey::InputType::Vni;
    assert_eq!(engine.type_string("diu91 "), "đíu ");
}
