use chess_notation::{parse_fen, CastlingRights, Color, Piece, PieceKind, Square};

fn sq(s: &str) -> Square {
    Square::from_str(s).expect("valid square literal in test")
}

fn white(kind: PieceKind) -> Piece {
    Piece {
        kind,
        color: Color::White,
    }
}

fn black(kind: PieceKind) -> Piece {
    Piece {
        kind,
        color: Color::Black,
    }
}

#[test]
fn parses_starting_position() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let pos = parse_fen(fen).expect("starting position should parse");

    assert_eq!(pos.piece_at(sq("a1")), Some(white(PieceKind::Rook)));
    assert_eq!(pos.piece_at(sq("e1")), Some(white(PieceKind::King)));
    assert_eq!(pos.piece_at(sq("d8")), Some(black(PieceKind::Queen)));
    assert_eq!(pos.piece_at(sq("e2")), Some(white(PieceKind::Pawn)));
    assert_eq!(pos.piece_at(sq("e4")), None);
    assert_eq!(pos.side_to_move, Color::White);
    assert_eq!(
        pos.castling,
        CastlingRights {
            white_kingside: true,
            white_queenside: true,
            black_kingside: true,
            black_queenside: true,
        }
    );
    assert_eq!(pos.en_passant, None);
    assert_eq!(pos.halfmove_clock, 0);
    assert_eq!(pos.fullmove_number, 1);
}

#[test]
fn round_trips_through_display() {
    let cases = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
        "r3k2r/8/8/8/8/8/8/R3K2R w Kq - 12 34",
        "8/8/8/8/8/8/8/8 w - - 0 1",
    ];
    for fen in cases {
        let pos = parse_fen(fen).expect("input should parse");
        assert_eq!(pos.to_string(), fen, "round trip mismatch for '{}'", fen);
    }
}

#[test]
fn en_passant_square_after_double_push() {
    // 1. e4, so black is to move and the target square is behind the pawn
    // on e3.
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1";
    let pos = parse_fen(fen).unwrap();
    assert_eq!(pos.en_passant, Some(sq("e3")));
    assert_eq!(pos.piece_at(sq("e4")), Some(white(PieceKind::Pawn)));
}

#[test]
fn rejects_malformed_fen() {
    let cases = [
        // Wrong number of top-level fields.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq -",
        // Wrong number of ranks.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP w KQkq - 0 1",
        // Rank doesn't add up to 8 files.
        "rnbqkbnr/pppppppp/8/8/8/7/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "rnbqkbnr/pppppppp/8/8/8/9/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        // Not a real piece letter.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPXP/RNBQKBNR w KQkq - 0 1",
        // Bad side to move.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR x KQkq - 0 1",
        // Bad castling letters.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkqx - 0 1",
        // En passant square on the wrong rank for the side to move.
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e6 0 1",
        // Non-square en passant field.
        "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq z9 0 1",
        // Non-numeric counters.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - x 1",
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 x",
        // Fullmove number must be at least 1.
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 0",
    ];
    for fen in cases {
        assert!(parse_fen(fen).is_err(), "expected '{}' to be rejected", fen);
    }
}
