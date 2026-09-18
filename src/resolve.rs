//! Resolving a parsed SAN `Move` against a `Position`: finding which piece
//! on the board a move actually refers to.
//!
//! This checks piece movement geometry, blocking pieces on sliding moves,
//! whether a disambiguation narrows the candidates to exactly one piece,
//! whether a capture/non-capture move matches what's actually on the
//! destination square, and whether making the move would leave the mover's
//! own king in check (including a king castling out of, through, or into
//! check).

use std::fmt;

use crate::{Color, Disambiguation, Move, Piece, PieceKind, Position, Square};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolveError {
    pub message: String,
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ResolveError {}

fn err(msg: impl Into<String>) -> ResolveError {
    ResolveError {
        message: msg.into(),
    }
}

/// The squares a resolved move touches. `from` is the moving piece's
/// origin; for castling it's the king's home square, regardless of which
/// rook is involved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedMove {
    pub from: Square,
    pub to: Square,
}

/// Find which piece on `pos` a parsed SAN move refers to, using the side to
/// move recorded on `pos`.
pub fn resolve_move(pos: &Position, mv: &Move) -> Result<ResolvedMove, ResolveError> {
    match mv {
        Move::Castle { kingside, .. } => resolve_castle(pos, *kingside),
        Move::Piece {
            piece,
            disambiguation,
            capture,
            to,
            ..
        } => resolve_piece_move(pos, *piece, *disambiguation, *capture, *to),
    }
}

fn resolve_castle(pos: &Position, kingside: bool) -> Result<ResolvedMove, ResolveError> {
    let color = pos.side_to_move;
    let home_rank = match color {
        Color::White => 0,
        Color::Black => 7,
    };
    let has_right = match (color, kingside) {
        (Color::White, true) => pos.castling.white_kingside,
        (Color::White, false) => pos.castling.white_queenside,
        (Color::Black, true) => pos.castling.black_kingside,
        (Color::Black, false) => pos.castling.black_queenside,
    };
    if !has_right {
        return Err(err(format!(
            "{} has no {} castling rights",
            color_name(color),
            if kingside { "kingside" } else { "queenside" }
        )));
    }

    let king_from = Square {
        file: 4,
        rank: home_rank,
    };
    let rook_file = if kingside { 7 } else { 0 };
    let rook_from = Square {
        file: rook_file,
        rank: home_rank,
    };
    let king_to = Square {
        file: if kingside { 6 } else { 2 },
        rank: home_rank,
    };

    if pos.piece_at(king_from) != Some(Piece { kind: PieceKind::King, color }) {
        return Err(err(format!("no king on {} to castle", king_from)));
    }
    if pos.piece_at(rook_from) != Some(Piece { kind: PieceKind::Rook, color }) {
        return Err(err(format!("no rook on {} to castle with", rook_from)));
    }

    // The squares strictly between king and rook must be empty; which side
    // of the king the rook sits on decides which run of files that is.
    let (low, high) = if rook_file < 4 {
        (rook_file + 1, 4)
    } else {
        (5, rook_file)
    };
    for file in low..high {
        let sq = Square { file, rank: home_rank };
        if pos.piece_at(sq).is_some() {
            return Err(err(format!("castling path is blocked on {}", sq)));
        }
    }

    // A king can't castle out of, through, or into check - only the three
    // squares it actually crosses matter, not the rook's side of the path.
    let opponent = other_color(color);
    let step: i8 = if kingside { 1 } else { -1 };
    for step_count in 0..3 {
        let file = 4 + step * step_count;
        let sq = Square { file: file as u8, rank: home_rank };
        if square_attacked_by(&pos.board, sq, opponent) {
            return Err(err(format!(
                "cannot castle through an attacked square ({})",
                sq
            )));
        }
    }

    Ok(ResolvedMove {
        from: king_from,
        to: king_to,
    })
}

fn other_color(color: Color) -> Color {
    match color {
        Color::White => Color::Black,
        Color::Black => Color::White,
    }
}

fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "white",
        Color::Black => "black",
    }
}

fn resolve_piece_move(
    pos: &Position,
    piece: PieceKind,
    disambiguation: Disambiguation,
    capture: bool,
    to: Square,
) -> Result<ResolvedMove, ResolveError> {
    if piece == PieceKind::Pawn {
        return resolve_pawn_move(pos, disambiguation, capture, to);
    }

    check_destination(pos, pos.side_to_move, capture, to)?;

    let color = pos.side_to_move;
    let mut reachable = Vec::new();
    let mut legal = Vec::new();
    for rank in 0..8u8 {
        for file in 0..8u8 {
            let from = Square { file, rank };
            if from == to {
                continue;
            }
            if pos.piece_at(from) != Some(Piece { kind: piece, color }) {
                continue;
            }
            if !matches_disambiguation(from, disambiguation) {
                continue;
            }
            if reaches(pos, piece, from, to) {
                reachable.push(from);
                if !leaves_king_in_check(pos, from, to, None) {
                    legal.push(from);
                }
            }
        }
    }

    finish(reachable, legal, piece, to)
}

fn check_destination(
    pos: &Position,
    mover: Color,
    capture: bool,
    to: Square,
) -> Result<(), ResolveError> {
    match pos.piece_at(to) {
        Some(occupant) if occupant.color == mover => {
            Err(err(format!("{} is occupied by your own piece", to)))
        }
        Some(_) if !capture => Err(err(format!(
            "{} is occupied; the move must be written as a capture",
            to
        ))),
        None if capture => Err(err(format!("no piece to capture on {}", to))),
        _ => Ok(()),
    }
}

fn matches_disambiguation(from: Square, disambiguation: Disambiguation) -> bool {
    match disambiguation {
        Disambiguation::None => true,
        Disambiguation::File(file) => from.file == file,
        Disambiguation::Rank(rank) => from.rank == rank,
        Disambiguation::Square(sq) => from == sq,
    }
}

fn reaches(pos: &Position, piece: PieceKind, from: Square, to: Square) -> bool {
    let df = to.file as i8 - from.file as i8;
    let dr = to.rank as i8 - from.rank as i8;

    match piece {
        PieceKind::Knight => matches!((df.abs(), dr.abs()), (1, 2) | (2, 1)),
        PieceKind::King => df.abs() <= 1 && dr.abs() <= 1,
        PieceKind::Bishop => df.abs() == dr.abs() && path_clear(pos, from, to),
        PieceKind::Rook => (df == 0 || dr == 0) && path_clear(pos, from, to),
        PieceKind::Queen => {
            (df.abs() == dr.abs() || df == 0 || dr == 0) && path_clear(pos, from, to)
        }
        PieceKind::Pawn => false, // pawns are resolved separately
    }
}

// Walks the squares strictly between `from` and `to` along the line the
// caller has already established is a straight rank, file, or diagonal.
fn path_clear(pos: &Position, from: Square, to: Square) -> bool {
    let step_file = (to.file as i8 - from.file as i8).signum();
    let step_rank = (to.rank as i8 - from.rank as i8).signum();
    let mut file = from.file as i8 + step_file;
    let mut rank = from.rank as i8 + step_rank;
    while (file, rank) != (to.file as i8, to.rank as i8) {
        let sq = Square {
            file: file as u8,
            rank: rank as u8,
        };
        if pos.piece_at(sq).is_some() {
            return false;
        }
        file += step_file;
        rank += step_rank;
    }
    true
}

fn resolve_pawn_move(
    pos: &Position,
    disambiguation: Disambiguation,
    capture: bool,
    to: Square,
) -> Result<ResolvedMove, ResolveError> {
    let color = pos.side_to_move;
    let dir: i8 = match color {
        Color::White => 1,
        Color::Black => -1,
    };
    let start_rank: u8 = match color {
        Color::White => 1,
        Color::Black => 6,
    };

    let mut reachable = Vec::new();
    let mut legal = Vec::new();

    if capture {
        let is_en_passant = pos.en_passant == Some(to);
        match pos.piece_at(to) {
            Some(occupant) if occupant.color == color => {
                return Err(err(format!("{} is occupied by your own piece", to)));
            }
            Some(_) => {}
            None if is_en_passant => {}
            None => return Err(err(format!("no piece to capture on {}", to))),
        }

        for df in [-1i8, 1i8] {
            let file = to.file as i8 + df;
            let rank = to.rank as i8 - dir;
            if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                continue;
            }
            let from = Square {
                file: file as u8,
                rank: rank as u8,
            };
            if pos.piece_at(from) == Some(Piece { kind: PieceKind::Pawn, color })
                && matches_disambiguation(from, disambiguation)
            {
                reachable.push(from);
                // An en passant capture removes a pawn that isn't sitting on
                // the destination square, so the simulated board needs to
                // know about it separately from the from/to move itself.
                let en_passant_capture = if is_en_passant {
                    Some(Square { file: to.file, rank: from.rank })
                } else {
                    None
                };
                if !leaves_king_in_check(pos, from, to, en_passant_capture) {
                    legal.push(from);
                }
            }
        }
    } else {
        if pos.piece_at(to).is_some() {
            return Err(err(format!(
                "{} is occupied; the move must be written as a capture",
                to
            )));
        }

        let one_back = to.rank as i8 - dir;
        if (0..8).contains(&one_back) {
            let from = Square {
                file: to.file,
                rank: one_back as u8,
            };
            if pos.piece_at(from) == Some(Piece { kind: PieceKind::Pawn, color })
                && matches_disambiguation(from, disambiguation)
            {
                reachable.push(from);
                if !leaves_king_in_check(pos, from, to, None) {
                    legal.push(from);
                }
            }
        }

        let two_back = to.rank as i8 - 2 * dir;
        if two_back >= 0 && two_back as u8 == start_rank {
            let mid = Square {
                file: to.file,
                rank: one_back as u8,
            };
            let from = Square {
                file: to.file,
                rank: start_rank,
            };
            if pos.piece_at(from) == Some(Piece { kind: PieceKind::Pawn, color })
                && pos.piece_at(mid).is_none()
                && matches_disambiguation(from, disambiguation)
            {
                reachable.push(from);
                if !leaves_king_in_check(pos, from, to, None) {
                    legal.push(from);
                }
            }
        }
    }

    finish(reachable, legal, PieceKind::Pawn, to)
}

// `reachable` is every piece of the right kind whose geometry and
// disambiguation match; `legal` is the subset of those that don't leave the
// mover's own king in check. Keeping both lets the error message tell a
// pinned piece apart from a piece that simply can't make the move at all.
fn finish(
    reachable: Vec<Square>,
    legal: Vec<Square>,
    piece: PieceKind,
    to: Square,
) -> Result<ResolvedMove, ResolveError> {
    match legal.len() {
        0 if reachable.is_empty() => Err(err(format!("no {:?} can reach {}", piece, to))),
        0 => Err(err(format!(
            "moving {} to {} would leave the king in check",
            reachable
                .iter()
                .map(|sq| sq.to_string())
                .collect::<Vec<_>>()
                .join(" or "),
            to
        ))),
        1 => Ok(ResolvedMove {
            from: legal[0],
            to,
        }),
        _ => Err(err(format!(
            "ambiguous move: {} pieces can reach {} ({})",
            legal.len(),
            to,
            legal
                .iter()
                .map(|sq| sq.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

// Applies a move to a scratch copy of the board and checks whether the
// mover's own king ends up attacked. `en_passant_capture`, when set, is the
// square of a pawn captured en passant - it isn't `to`, so it has to be
// cleared separately from the from/to move.
fn leaves_king_in_check(
    pos: &Position,
    from: Square,
    to: Square,
    en_passant_capture: Option<Square>,
) -> bool {
    let mover = pos.side_to_move;
    let mut board = pos.board;
    board[square_index(to)] = board[square_index(from)];
    board[square_index(from)] = None;
    if let Some(sq) = en_passant_capture {
        board[square_index(sq)] = None;
    }

    match king_square(&board, mover) {
        Some(king_sq) => square_attacked_by(&board, king_sq, other_color(mover)),
        // A position with no king for the side to move can't happen from a
        // legally parsed FEN, but treating it as "not in check" rather than
        // panicking keeps this function total.
        None => false,
    }
}

fn square_index(sq: Square) -> usize {
    sq.rank as usize * 8 + sq.file as usize
}

fn king_square(board: &[Option<Piece>; 64], color: Color) -> Option<Square> {
    board
        .iter()
        .position(|p| *p == Some(Piece { kind: PieceKind::King, color }))
        .map(|i| Square {
            file: (i % 8) as u8,
            rank: (i / 8) as u8,
        })
}

/// Whether `sq` is attacked by any piece of `attacker`'s color on `board`.
fn square_attacked_by(board: &[Option<Piece>; 64], sq: Square, attacker: Color) -> bool {
    let piece_at = |s: Square| board[square_index(s)];

    // Pawns attack diagonally toward the opponent, i.e. opposite the
    // direction they push, so look one rank behind `sq` from `attacker`'s
    // point of view.
    let pawn_dir: i8 = match attacker {
        Color::White => 1,
        Color::Black => -1,
    };
    for df in [-1i8, 1i8] {
        let file = sq.file as i8 + df;
        let rank = sq.rank as i8 - pawn_dir;
        if (0..8).contains(&file) && (0..8).contains(&rank) {
            let from = Square { file: file as u8, rank: rank as u8 };
            if piece_at(from) == Some(Piece { kind: PieceKind::Pawn, color: attacker }) {
                return true;
            }
        }
    }

    const KNIGHT_STEPS: [(i8, i8); 8] = [
        (1, 2), (2, 1), (-1, 2), (-2, 1), (1, -2), (2, -1), (-1, -2), (-2, -1),
    ];
    for (df, dr) in KNIGHT_STEPS {
        let file = sq.file as i8 + df;
        let rank = sq.rank as i8 + dr;
        if (0..8).contains(&file) && (0..8).contains(&rank) {
            let from = Square { file: file as u8, rank: rank as u8 };
            if piece_at(from) == Some(Piece { kind: PieceKind::Knight, color: attacker }) {
                return true;
            }
        }
    }

    for df in -1i8..=1 {
        for dr in -1i8..=1 {
            if df == 0 && dr == 0 {
                continue;
            }
            let file = sq.file as i8 + df;
            let rank = sq.rank as i8 + dr;
            if (0..8).contains(&file) && (0..8).contains(&rank) {
                let from = Square { file: file as u8, rank: rank as u8 };
                if piece_at(from) == Some(Piece { kind: PieceKind::King, color: attacker }) {
                    return true;
                }
            }
        }
    }

    const DIRECTIONS: [(i8, i8); 8] = [
        (1, 1), (1, -1), (-1, 1), (-1, -1), (1, 0), (-1, 0), (0, 1), (0, -1),
    ];
    for (df, dr) in DIRECTIONS {
        let diagonal = df != 0 && dr != 0;
        let mut file = sq.file as i8 + df;
        let mut rank = sq.rank as i8 + dr;
        while (0..8).contains(&file) && (0..8).contains(&rank) {
            let at = Square { file: file as u8, rank: rank as u8 };
            if let Some(p) = piece_at(at) {
                if p.color == attacker {
                    let slides_this_way = if diagonal {
                        p.kind == PieceKind::Bishop || p.kind == PieceKind::Queen
                    } else {
                        p.kind == PieceKind::Rook || p.kind == PieceKind::Queen
                    };
                    if slides_this_way {
                        return true;
                    }
                }
                break;
            }
            file += df;
            rank += dr;
        }
    }

    false
}
