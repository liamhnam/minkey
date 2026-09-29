// Behavioural parity with the original OpenKey engine.
// tests/parity/openkey_parity.tsv holds "<cfg>|<keys>\t<OpenKey output>" rows produced by
// tests/parity/ok_harness.cpp (see tests/parity/regen.sh). This test types the same keys into
// Minkey's engine with the exact same simulation and expects identical text.

use minkey::tables::key_code_to_character;
use minkey::types::*;
use minkey::VietnameseEngine;

fn map_char(ch: char) -> (u16, bool) {
    match ch {
        'a'..='z' => (ch.to_ascii_uppercase() as u16, false),
        'A'..='Z' => (ch as u16, true),
        '0'..='9' => (ch as u16, false),
        ' ' => (KEY_SPACE, false),
        '@' => (KEY_DELETE, false), // backspace token
        '[' => (KEY_LEFT_BRACKET, false),
        '{' => (KEY_LEFT_BRACKET, true),
        ']' => (KEY_RIGHT_BRACKET, false),
        '}' => (KEY_RIGHT_BRACKET, true),
        '.' => (KEY_DOT, false),
        ',' => (KEY_COMMA, false),
        ';' => (KEY_SEMICOLON, false),
        '\'' => (KEY_QUOTE, false),
        '/' => (KEY_SLASH, false),
        '-' => (KEY_MINUS, false),
        '?' => (KEY_SLASH, true),
        '!' => (KEY_1, true),
        _ => (KEY_SPACE, false),
    }
}

/// Mirrors ok_harness.cpp: apply each hook result to a text buffer
fn simulate(eng: &mut VietnameseEngine, cfg: &str, text: &str) -> String {
    let c: Vec<u32> = cfg.split_whitespace().map(|x| x.parse().unwrap()).collect();
    eng.input_type = InputType::from_u32(c[0]);
    eng.use_modern_orthography = c[1] != 0;
    eng.set_check_spelling(c[2] != 0);
    eng.restore_if_wrong_spelling = c[3] != 0;
    eng.quick_telex = c[4] != 0;
    eng.allow_consonant_zfwj = c[5] != 0;
    eng.quick_start_consonant = c[6] != 0;
    eng.quick_end_consonant = c[7] != 0;
    eng.free_mark = c[8] != 0;
    eng.start_new_session();
    eng.handle_event(KeyEvent::Mouse, KeyEventState::MouseDown, 0, 0, false);

    let mut out: Vec<char> = Vec::new();
    for ch in text.chars() {
        let (key, caps) = map_char(ch);
        let st = eng
            .handle_event(KeyEvent::Keyboard, KeyEventState::KeyDown, key, caps as u8, false)
            .clone();
        match st.code {
            HookCodeState::DoNothing | HookCodeState::BreakWord => {
                if key == KEY_DELETE {
                    out.pop();
                } else {
                    out.push(ch);
                }
            }
            HookCodeState::WillProcess | HookCodeState::Restore | HookCodeState::RestoreAndStartNewSession => {
                for _ in 0..st.backspace_count {
                    out.pop();
                }
                for i in (0..st.new_char_count as usize).rev() {
                    let d = st.char_data[i];
                    let u = if d & (PURE_CHARACTER_MASK | CHAR_CODE_MASK) != 0 {
                        d & 0xFFFF
                    } else {
                        key_code_to_character(d) as u32
                    };
                    out.push(char::from_u32(u).unwrap_or('\u{FFFD}'));
                }
                if st.code != HookCodeState::WillProcess {
                    out.push(ch);
                }
                if st.code == HookCodeState::RestoreAndStartNewSession {
                    eng.start_new_session();
                }
            }
            HookCodeState::ReplaceMacro => {}
        }
    }
    out.into_iter().collect()
}

/// Rows where Minkey intentionally differs because OpenKey is wrong:
/// - OpenKey compares `TypingWord[..] == KEY_T` including the caps bit, so the "thuơ" rule is
///   skipped when "T" is capitalised ("Thuowr" gives "Thưở" instead of "Thuở").
/// - With old-style marks, "qu" + tone and no other vowel puts the mark on "q" (not a character).
const INTENTIONAL_DIFFS: &[(&str, &str)] = &[
    ("3 0 0 0 1 0 0 1 0|hqu7@nsa.4sjd.i/iEq", "hqúna.4sjd.i/iEq"),
    ("2 0 0 0 1 1 0 1 0|esO?qunFUl@.sm[cfy", "éO?qùnU.sm[cfy"),
    ("2 1 1 0 1 0 0 0 0|Sopjo ghyeuce Thuowf soow poayi", "Sộp ghyeuce Thuờ sơ poayi"),
];

/// OpenKey sometimes emits raw engine data that is not a character (shown as NUL here).
/// Minkey must emit a real character in those positions and match everywhere else.
fn matches_allowing_openkey_nul(expected: &str, got: &str) -> bool {
    let e: Vec<char> = expected.chars().collect();
    let g: Vec<char> = got.chars().collect();
    e.len() == g.len() && e.iter().zip(&g).all(|(a, b)| *b != '\0' && (a == b || *a == '\0'))
}

#[test]
fn matches_openkey_engine() {
    let data = include_str!("parity/openkey_parity.tsv");
    // Larger groups of reviewed differences, one row per case (reasons in the file header)
    let reviewed: std::collections::HashMap<&str, &str> = include_str!("parity/minkey_intentional_diffs.tsv")
        .lines()
        .filter(|l| !l.starts_with('#'))
        .filter_map(|l| l.split_once('\t'))
        .collect();
    let mut eng = VietnameseEngine::new();
    eng.use_macro = false;

    let mut total = 0;
    let mut failures = Vec::new();
    for line in data.lines() {
        let (input, openkey) = line.split_once('\t').expect("tab separated row");
        let (cfg, keys) = input.split_once('|').expect("cfg|keys");
        let got = simulate(&mut eng, cfg, keys);
        total += 1;

        let ok = match INTENTIONAL_DIFFS.iter().find(|(i, _)| *i == input) {
            Some((_, minkey)) => got == *minkey,
            None if reviewed.contains_key(input) => got == reviewed[input],
            None if openkey.contains('\0') => matches_allowing_openkey_nul(openkey, &got),
            None => got == openkey,
        };
        if !ok {
            failures.push(format!("  [{cfg}] {keys:?}\n    openkey: {openkey:?}\n    minkey:  {got:?}"));
        }
    }

    assert!(
        failures.is_empty(),
        "{} / {} cases differ from OpenKey. First ones:\n{}",
        failures.len(),
        total,
        failures.iter().take(15).cloned().collect::<Vec<_>>().join("\n")
    );
}
