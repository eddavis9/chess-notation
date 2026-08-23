use chess_notation::{parse_san, CheckState, Disambiguation, Move, PieceKind, Square};

fn sq(s: &str) -> Square {
    Square::from_str(s).expect("valid square literal in test table")
}

#[test]
fn parses_valid_moves() {
    let cases: Vec<(&str, Move)> = vec![
        (
            "e4",
            Move::Piece {
                piece: PieceKind::Pawn,
                disambiguation: Disambiguation::None,
                capture: false,
                to: sq("e4"),
                promotion: None,
                check: CheckState::None,
            },
        ),
        (
            "exd5",
            Move::Piece {
                piece: PieceKind::Pawn,
                disambiguation: Disambiguation::File(4),
                capture: true,
                to: sq("d5"),
                promotion: None,
                check: CheckState::None,
            },
        ),
        (
            // Two knights can both reach d7; disambiguate by file.
            "Nbd7",
            Move::Piece {
                piece: PieceKind::Knight,
                disambiguation: Disambiguation::File(1),
                capture: false,
                to: sq("d7"),
                promotion: None,
                check: CheckState::None,
            },
        ),
        (
            // Same idea, but the file is shared so disambiguate by rank.
            "N1d3",
            Move::Piece {
                piece: PieceKind::Knight,
                disambiguation: Disambiguation::Rank(0),
                capture: false,
                to: sq("d3"),
                promotion: None,
                check: CheckState::None,
            },
        ),
        (
            // Neither file nor rank alone is unique, so the full square is given.
            "Qh4xe1",
            Move::Piece {
                piece: PieceKind::Queen,
                disambiguation: Disambiguation::Square(sq("h4")),
                capture: true,
                to: sq("e1"),
                promotion: None,
                check: CheckState::None,
            },
        ),
        (
            "e8=Q",
            Move::Piece {
                piece: PieceKind::Pawn,
                disambiguation: Disambiguation::None,
                capture: false,
                to: sq("e8"),
                promotion: Some(PieceKind::Queen),
                check: CheckState::None,
            },
        ),
        (
            // Capturing promotion with check, all in one token.
            "exd8=Q+",
            Move::Piece {
                piece: PieceKind::Pawn,
                disambiguation: Disambiguation::File(4),
                capture: true,
                to: sq("d8"),
                promotion: Some(PieceKind::Queen),
                check: CheckState::Check,
            },
        ),
        (
            "Qxe1#",
            Move::Piece {
                piece: PieceKind::Queen,
                disambiguation: Disambiguation::None,
                capture: true,
                to: sq("e1"),
                promotion: None,
                check: CheckState::Checkmate,
            },
        ),
        (
            "O-O",
            Move::Castle {
                kingside: true,
                check: CheckState::None,
            },
        ),
        (
            "O-O-O",
            Move::Castle {
                kingside: false,
                check: CheckState::None,
            },
        ),
        (
            // Digit zero instead of the letter O, seen in some old databases.
            "0-0",
            Move::Castle {
                kingside: true,
                check: CheckState::None,
            },
        ),
        (
            "O-O+",
            Move::Castle {
                kingside: true,
                check: CheckState::Check,
            },
        ),
        (
            "O-O-O#",
            Move::Castle {
                kingside: false,
                check: CheckState::Checkmate,
            },
        ),
    ];

    for (input, expected) in cases {
        let actual = parse_san(input)
            .unwrap_or_else(|e| panic!("expected '{}' to parse, got error: {}", input, e));
        assert_eq!(actual, expected, "mismatch parsing '{}'", input);
    }
}

#[test]
fn rejects_malformed_moves() {
    let cases: Vec<&str> = vec![
        "",
        "   ",
        "e",
        "e9",
        // Pawn captures must name the origin file.
        "xe4",
        "Nx",
        // Three disambiguation characters is never valid SAN.
        "Qh4d5xe1",
        // Promotion only makes sense on the back rank.
        "e4=Q",
        // Not a real promotion piece.
        "e8=P",
        "e8=",
        // Garbage input containing multi-byte characters must be rejected
        // cleanly rather than panicking on a bad slice index.
        "\u{e9}4",
        "e8=\u{e7}",
        "\u{e9}\u{e9}\u{e9}=Q",
    ];

    for input in cases {
        assert!(
            parse_san(input).is_err(),
            "expected '{}' to be rejected",
            input
        );
    }
}

#[test]
fn round_trips_through_display() {
    let cases = [
        "e4", "exd5", "Nbd7", "N1d3", "Qh4xe1", "e8=Q", "O-O", "O-O-O", "Qxe1#",
    ];
    for input in cases {
        let parsed = parse_san(input).expect("input should parse");
        assert_eq!(
            parsed.to_string(),
            input,
            "round trip mismatch for '{}'",
            input
        );
    }
}
