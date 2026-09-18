use chess_notation::{parse_fen, parse_san, resolve_move, Square};

fn sq(s: &str) -> Square {
    Square::from_str(s).expect("valid square literal in test")
}

fn resolve(fen: &str, mv: &str) -> Result<(Square, Square), String> {
    let pos = parse_fen(fen).expect("valid FEN in test");
    let mv = parse_san(mv).expect("valid SAN in test");
    resolve_move(&pos, &mv)
        .map(|r| (r.from, r.to))
        .map_err(|e| e.to_string())
}

#[test]
fn pawn_single_push_from_starting_rank() {
    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let (from, to) = resolve(fen, "e4").unwrap();
    assert_eq!(from, sq("e2"));
    assert_eq!(to, sq("e4"));
}

#[test]
fn pawn_single_push_after_double_push() {
    // A pawn already on e4 can only push one square, to e5.
    let fen = "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1";
    let (from, to) = resolve(fen, "e5").unwrap();
    assert_eq!(from, sq("e4"));
    assert_eq!(to, sq("e5"));
}

#[test]
fn pawn_double_push_blocked_by_intervening_piece() {
    // Destination e4 is open, but the knight on e3 blocks the pawn from
    // passing through on its way from e2.
    let fen = "rnbqkbnr/pppppppp/8/8/8/4n3/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    assert!(resolve(fen, "e4").is_err());
}

#[test]
fn pawn_capture_requires_target_piece() {
    let fen = "rnbqkbnr/ppp1pppp/8/3p4/4P3/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1";
    let (from, to) = resolve(fen, "exd5").unwrap();
    assert_eq!(from, sq("e4"));
    assert_eq!(to, sq("d5"));
}

#[test]
fn pawn_capture_without_target_is_rejected() {
    let fen = "rnbqkbnr/ppppp1pp/8/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 0 1";
    assert!(resolve(fen, "exd5").is_err());
}

#[test]
fn pawn_en_passant_capture() {
    let fen = "rnbqkbnr/ppp1pppp/8/3pP3/8/8/PPPP1PPP/RNBQKBNR w KQkq d6 0 1";
    let (from, to) = resolve(fen, "exd6").unwrap();
    assert_eq!(from, sq("e5"));
    assert_eq!(to, sq("d6"));
}

#[test]
fn knight_disambiguation_by_file() {
    let fen = "8/8/8/8/8/8/8/N1N1K2k w - - 0 1";
    let (from, to) = resolve(fen, "Nab3").unwrap();
    assert_eq!(from, sq("a1"));
    assert_eq!(to, sq("b3"));
}

#[test]
fn knight_ambiguous_without_disambiguation_is_rejected() {
    let fen = "8/8/8/8/8/8/8/N1N1K2k w - - 0 1";
    assert!(resolve(fen, "Nb3").is_err());
}

#[test]
fn bishop_move_blocked_by_intervening_piece() {
    // Bishop on a1, target on f6, both on the long diagonal, but a pawn on
    // c3 sits between them.
    let fen = "8/8/5p2/8/8/2P5/8/B3K2k w - - 0 1";
    assert!(resolve(fen, "Bxf6").is_err());
}

#[test]
fn rook_slides_along_clear_rank() {
    let fen = "8/8/8/8/8/8/8/R3K2k w - - 0 1";
    let (from, to) = resolve(fen, "Rd1").unwrap();
    assert_eq!(from, sq("a1"));
    assert_eq!(to, sq("d1"));
}

#[test]
fn cannot_capture_own_piece() {
    let fen = "8/8/8/8/8/8/8/R2NK2k w - - 0 1";
    assert!(resolve(fen, "Rxd1").is_err());
}

#[test]
fn non_capture_onto_occupied_square_is_rejected() {
    let fen = "8/8/8/8/8/8/8/R2nK2k w - - 0 1";
    assert!(resolve(fen, "Rd1").is_err());
}

#[test]
fn castles_kingside_when_path_is_clear() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let (from, to) = resolve(fen, "O-O").unwrap();
    assert_eq!(from, sq("e1"));
    assert_eq!(to, sq("g1"));
}

#[test]
fn castles_queenside_when_path_is_clear() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1";
    let (from, to) = resolve(fen, "O-O-O").unwrap();
    assert_eq!(from, sq("e1"));
    assert_eq!(to, sq("c1"));
}

#[test]
fn castling_without_rights_is_rejected() {
    let fen = "r3k2r/8/8/8/8/8/8/R3K2R w kq - 0 1";
    assert!(resolve(fen, "O-O").is_err());
}

#[test]
fn castling_through_blocked_path_is_rejected() {
    let fen = "r3k2r/8/8/8/8/8/8/R2NK2R w KQkq - 0 1";
    assert!(resolve(fen, "O-O-O").is_err());
}

#[test]
fn king_cannot_move_into_an_attacked_square() {
    // The rook on a2 rakes the whole second rank, so e2 is off limits even
    // though nothing stands between the king and it.
    let fen = "8/8/8/8/8/8/r7/4K2k w - - 0 1";
    assert!(resolve(fen, "Ke2").is_err());
}

#[test]
fn pinned_piece_cannot_move_off_the_pin_line() {
    // The rook on e2 is the only thing between the king and the rook on
    // e8; sidestepping to d2 would expose the king down the e-file.
    let fen = "4r2k/8/8/8/8/8/4R3/4K3 w - - 0 1";
    assert!(resolve(fen, "Rd2").is_err());
}

#[test]
fn pinned_piece_can_still_move_along_the_pin_line() {
    let fen = "4r2k/8/8/8/8/8/4R3/4K3 w - - 0 1";
    let (from, to) = resolve(fen, "Re5").unwrap();
    assert_eq!(from, sq("e2"));
    assert_eq!(to, sq("e5"));
}

#[test]
fn cannot_castle_out_of_check() {
    let fen = "4r2k/8/8/8/8/8/8/R3K2R w KQ - 0 1";
    assert!(resolve(fen, "O-O").is_err());
}

#[test]
fn cannot_castle_through_an_attacked_square() {
    // The rook on f8 covers f1, one of the squares the king crosses on its
    // way to g1.
    let fen = "5r1k/8/8/8/8/8/8/R3K2R w KQ - 0 1";
    assert!(resolve(fen, "O-O").is_err());
}

#[test]
fn en_passant_capture_that_exposes_king_is_rejected() {
    // Capturing en passant clears both the c5 and d5 pawns off the fifth
    // rank in the same move, opening a direct line from the rook on a5 to
    // the king on e5.
    let fen = "7k/8/8/r1pPK3/8/8/8/8 w - c6 0 1";
    assert!(resolve(fen, "dxc6").is_err());
}
