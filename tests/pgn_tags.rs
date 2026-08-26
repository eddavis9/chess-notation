use chess_notation::{parse_tag_pairs, TagPair};

fn tag(name: &str, value: &str) -> TagPair {
    TagPair {
        name: name.to_string(),
        value: value.to_string(),
    }
}

#[test]
fn parses_seven_tag_roster() {
    let input = "\
[Event \"F/S Return Match\"]
[Site \"Belgrade, Serbia JUG\"]
[Date \"1992.11.04\"]
[Round \"29\"]
[White \"Fischer, Robert J.\"]
[Black \"Spassky, Boris V.\"]
[Result \"1/2-1/2\"]

1. e4 e5 2. Nf3 Nc6 1/2-1/2";

    let tags = parse_tag_pairs(input).expect("well-formed header should parse");
    assert_eq!(
        tags,
        vec![
            tag("Event", "F/S Return Match"),
            tag("Site", "Belgrade, Serbia JUG"),
            tag("Date", "1992.11.04"),
            tag("Round", "29"),
            tag("White", "Fischer, Robert J."),
            tag("Black", "Spassky, Boris V."),
            tag("Result", "1/2-1/2"),
        ]
    );
}

#[test]
fn stops_before_move_text() {
    let input = "[Event \"Casual Game\"]\n1. e4 e5";
    let tags = parse_tag_pairs(input).expect("header should parse");
    assert_eq!(tags, vec![tag("Event", "Casual Game")]);
}

#[test]
fn empty_input_has_no_tags() {
    assert_eq!(parse_tag_pairs("").unwrap(), Vec::<TagPair>::new());
}

#[test]
fn unescapes_backslash_and_quote() {
    let input = r#"[Annotator "Tal \"The Magician\" Mikhail"]"#;
    let tags = parse_tag_pairs(input).unwrap();
    assert_eq!(tags, vec![tag("Annotator", "Tal \"The Magician\" Mikhail")]);
}

#[test]
fn escaped_backslash_survives() {
    let input = r#"[Comment "C:\\games\\1.pgn"]"#;
    let tags = parse_tag_pairs(input).unwrap();
    assert_eq!(tags, vec![tag("Comment", r"C:\games\1.pgn")]);
}

#[test]
fn rejects_malformed_tag_lines() {
    let cases = [
        "[Event]",
        "[Event \"unterminated]",
        "[\"noname\"]",
        "[Ev ent \"spaced name\"]",
        "[Event \"value\" garbage]",
    ];
    for input in cases {
        assert!(
            parse_tag_pairs(input).is_err(),
            "expected '{}' to be rejected",
            input
        );
    }
}
