use chess_notation::{parse_movetext, parse_san, Move, MoveTextEntry};

fn mv(s: &str) -> Move {
    parse_san(s).unwrap()
}

fn plain_entry(s: &str) -> MoveTextEntry {
    MoveTextEntry {
        mv: mv(s),
        comment: None,
        nags: Vec::new(),
        variations: Vec::new(),
    }
}

#[test]
fn parses_plain_move_sequences() {
    let cases: &[(&str, &[&str], Option<&str>)] = &[
        ("1. e4 e5 2. Nf3 Nc6", &["e4", "e5", "Nf3", "Nc6"], None),
        ("1.e4 e5 2.Nf3 Nc6", &["e4", "e5", "Nf3", "Nc6"], None),
        (
            "1. e4 e5 2. Nf3 Nc6 1/2-1/2",
            &["e4", "e5", "Nf3", "Nc6"],
            Some("1/2-1/2"),
        ),
        ("1. e4 e5 2. Nf3 Nc6 1-0", &["e4", "e5", "Nf3", "Nc6"], Some("1-0")),
        ("1. e4 e5 2. Nf3 Nc6 0-1", &["e4", "e5", "Nf3", "Nc6"], Some("0-1")),
        ("1. e4 e5 2. Nf3 Nc6 *", &["e4", "e5", "Nf3", "Nc6"], Some("*")),
        ("", &[], None),
        ("1... e5 2. Nf3", &["e5", "Nf3"], None),
    ];

    for (input, moves, result) in cases {
        let parsed = parse_movetext(input).expect("well-formed move text should parse");
        let expected: Vec<MoveTextEntry> = moves.iter().map(|s| plain_entry(s)).collect();
        assert_eq!(parsed.moves, expected, "moves for '{}'", input);
        assert_eq!(
            parsed.result,
            result.map(|s| s.to_string()),
            "result for '{}'",
            input
        );
        assert_eq!(parsed.preamble_comment, None, "preamble for '{}'", input);
    }
}

#[test]
fn comments_attach_to_the_preceding_move() {
    let parsed = parse_movetext("1. e4 {best by test} e5").unwrap();
    assert_eq!(parsed.moves[0].comment.as_deref(), Some("best by test"));
    assert_eq!(parsed.moves[1].comment, None);
}

#[test]
fn adjacent_comments_are_joined_with_a_space() {
    let parsed = parse_movetext("1. e4 {a}{b} e5").unwrap();
    assert_eq!(parsed.moves[0].comment.as_deref(), Some("a b"));
}

#[test]
fn semicolon_comments_run_to_end_of_line() {
    let parsed = parse_movetext("1. e4 ; a quick remark\ne5").unwrap();
    assert_eq!(parsed.moves[0].comment.as_deref(), Some("a quick remark"));
    assert_eq!(parsed.moves[1].mv, mv("e5"));
}

#[test]
fn leading_comment_is_a_preamble_not_attached_to_a_move() {
    let parsed = parse_movetext("{game start} 1. e4").unwrap();
    assert_eq!(parsed.preamble_comment.as_deref(), Some("game start"));
    assert_eq!(parsed.moves[0].comment, None);
}

#[test]
fn nags_attach_to_the_preceding_move() {
    let parsed = parse_movetext("1. e4 $1 e5 $2 $10").unwrap();
    assert_eq!(parsed.moves[0].nags, vec![1]);
    assert_eq!(parsed.moves[1].nags, vec![2, 10]);
}

#[test]
fn variations_attach_to_the_move_they_replace() {
    let parsed = parse_movetext("1. e4 e5 2. Nf3 Nc6 (2... d6 3. d4) 3. Bb5").unwrap();
    assert_eq!(parsed.moves.len(), 5);
    assert!(parsed.moves[0].variations.is_empty());
    assert!(parsed.moves[1].variations.is_empty());
    assert!(parsed.moves[2].variations.is_empty());

    let variations = &parsed.moves[3].variations;
    assert_eq!(variations.len(), 1);
    assert_eq!(
        variations[0].iter().map(|e| e.mv).collect::<Vec<_>>(),
        vec![mv("d6"), mv("d4")]
    );
}

#[test]
fn nested_variations_are_supported() {
    let parsed = parse_movetext("1. e4 e5 (1... c5 (1... e6 2. d4) 2. Nf3) 2. Nf3").unwrap();
    let outer = &parsed.moves[1].variations;
    assert_eq!(outer.len(), 1);
    assert_eq!(outer[0][0].mv, mv("c5"));

    let inner = &outer[0][0].variations;
    assert_eq!(inner.len(), 1);
    assert_eq!(
        inner[0].iter().map(|e| e.mv).collect::<Vec<_>>(),
        vec![mv("e6"), mv("d4")]
    );
}

#[test]
fn rejects_malformed_move_text() {
    let cases = [
        "1. e4 (2. d4",        // unterminated variation
        "1. e4 {unterminated", // unterminated comment
        "1. e4)",              // unmatched close paren
        "(1. e4)",             // variation with nothing preceding it
        "$5 1. e4",             // NAG with nothing preceding it
        "1. e4 e9",             // malformed SAN move
        "1. e4 2-3",             // digit token that's neither move number nor result
    ];
    for input in cases {
        assert!(
            parse_movetext(input).is_err(),
            "expected '{}' to be rejected",
            input
        );
    }
}
