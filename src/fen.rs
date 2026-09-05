//! Parsing (and formatting) for Forsyth-Edwards Notation, the standard way
//! to record a chess position: piece placement, side to move, castling
//! rights, the en passant target square, and the two move counters.
//!
//! This gives SAN parsing somewhere to get board state from, but it doesn't
//! itself resolve a move against that state - that's the next piece to
//! build on top of this.

use std::fmt;

use crate::{err, ParseError, PieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Piece {
    pub kind: PieceKind,
    pub color: Color,
}

/// Which castling moves each side still has rights to, independent of
/// whether that move is legal in the current position (the rook or king
/// involved may since have moved into a position that blocks it).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CastlingRights {
    pub white_kingside: bool,
    pub white_queenside: bool,
    pub black_kingside: bool,
    pub black_queenside: bool,
}

impl CastlingRights {
    fn none() -> Self {
        CastlingRights::default()
    }

    fn any(&self) -> bool {
        self.white_kingside || self.white_queenside || self.black_kingside || self.black_queenside
    }
}

/// A parsed chess position. `board` is indexed by `rank * 8 + file`, the
/// same convention `Square` uses, so a piece at `sq` is `board[sq.rank as
/// usize * 8 + sq.file as usize]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub board: [Option<Piece>; 64],
    pub side_to_move: Color,
    pub castling: CastlingRights,
    pub en_passant: Option<Square>,
    pub halfmove_clock: u32,
    pub fullmove_number: u32,
}

impl Position {
    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.board[sq.rank as usize * 8 + sq.file as usize]
    }
}

fn piece_from_fen_letter(c: char) -> Option<Piece> {
    let kind = PieceKind::from_fen_letter(c.to_ascii_uppercase())?;
    let color = if c.is_ascii_uppercase() {
        Color::White
    } else {
        Color::Black
    };
    Some(Piece { kind, color })
}

fn piece_to_fen_letter(piece: Piece) -> char {
    let letter = piece.kind.to_letter();
    match piece.color {
        Color::White => letter,
        Color::Black => letter.to_ascii_lowercase(),
    }
}

/// Parse a FEN record, e.g. the starting position:
/// `rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1`.
pub fn parse_fen(input: &str) -> Result<Position, ParseError> {
    let fields: Vec<&str> = input.split_whitespace().collect();
    if fields.len() != 6 {
        return Err(err(format!(
            "FEN must have 6 space-separated fields, found {}",
            fields.len()
        )));
    }

    let board = parse_placement(fields[0])?;
    let side_to_move = parse_side_to_move(fields[1])?;
    let castling = parse_castling(fields[2])?;
    let en_passant = parse_en_passant(fields[3], side_to_move)?;
    let halfmove_clock = fields[4]
        .parse::<u32>()
        .map_err(|_| err(format!("invalid halfmove clock '{}'", fields[4])))?;
    let fullmove_number = fields[5]
        .parse::<u32>()
        .map_err(|_| err(format!("invalid fullmove number '{}'", fields[5])))?;
    if fullmove_number == 0 {
        return Err(err("fullmove number must be at least 1"));
    }

    Ok(Position {
        board,
        side_to_move,
        castling,
        en_passant,
        halfmove_clock,
        fullmove_number,
    })
}

fn parse_placement(field: &str) -> Result<[Option<Piece>; 64], ParseError> {
    let ranks: Vec<&str> = field.split('/').collect();
    if ranks.len() != 8 {
        return Err(err(format!(
            "piece placement must have 8 ranks, found {}",
            ranks.len()
        )));
    }

    let mut board = [None; 64];
    // FEN lists ranks from 8 down to 1, but our board is indexed from rank
    // 1 (index 0), so the first rank string fills rank index 7.
    for (i, rank_str) in ranks.iter().enumerate() {
        let rank = 7 - i as u8;
        let mut file = 0u8;
        for c in rank_str.chars() {
            if let Some(skip) = c.to_digit(10) {
                if skip == 0 || skip > 8 {
                    return Err(err(format!("invalid empty-square count '{}'", c)));
                }
                file += skip as u8;
            } else {
                let piece = piece_from_fen_letter(c)
                    .ok_or_else(|| err(format!("invalid piece letter '{}'", c)))?;
                if file >= 8 {
                    return Err(err(format!("rank '{}' has more than 8 files", rank_str)));
                }
                board[rank as usize * 8 + file as usize] = Some(piece);
                file += 1;
            }
            if file > 8 {
                return Err(err(format!("rank '{}' has more than 8 files", rank_str)));
            }
        }
        if file != 8 {
            return Err(err(format!(
                "rank '{}' does not account for exactly 8 files",
                rank_str
            )));
        }
    }

    Ok(board)
}

fn parse_side_to_move(field: &str) -> Result<Color, ParseError> {
    match field {
        "w" => Ok(Color::White),
        "b" => Ok(Color::Black),
        _ => Err(err(format!("invalid side to move '{}'", field))),
    }
}

fn parse_castling(field: &str) -> Result<CastlingRights, ParseError> {
    if field == "-" {
        return Ok(CastlingRights::none());
    }

    let mut rights = CastlingRights::none();
    for c in field.chars() {
        match c {
            'K' => rights.white_kingside = true,
            'Q' => rights.white_queenside = true,
            'k' => rights.black_kingside = true,
            'q' => rights.black_queenside = true,
            _ => return Err(err(format!("invalid castling availability '{}'", field))),
        }
    }
    Ok(rights)
}

fn parse_en_passant(field: &str, side_to_move: Color) -> Result<Option<Square>, ParseError> {
    if field == "-" {
        return Ok(None);
    }
    let sq = Square::from_str(field)
        .ok_or_else(|| err(format!("invalid en passant square '{}'", field)))?;

    // The target square sits behind the pawn that just made a two-square
    // move, so its rank is fixed by whose turn it now is: white to move
    // means black just pushed a pawn to rank 5, leaving the target on rank
    // 6, and vice versa.
    let expected_rank = match side_to_move {
        Color::White => 5, // rank 6
        Color::Black => 2, // rank 3
    };
    if sq.rank != expected_rank {
        return Err(err(format!(
            "en passant square '{}' is not on the expected rank for '{}' to move",
            field,
            if side_to_move == Color::White { "w" } else { "b" }
        )));
    }
    Ok(sq)
}

impl fmt::Display for Position {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for rank in (0..8).rev() {
            let mut empty_run = 0u8;
            for file in 0..8 {
                match self.board[rank as usize * 8 + file as usize] {
                    Some(piece) => {
                        if empty_run > 0 {
                            write!(f, "{}", empty_run)?;
                            empty_run = 0;
                        }
                        write!(f, "{}", piece_to_fen_letter(piece))?;
                    }
                    None => empty_run += 1,
                }
            }
            if empty_run > 0 {
                write!(f, "{}", empty_run)?;
            }
            if rank > 0 {
                write!(f, "/")?;
            }
        }

        write!(f, " {}", if self.side_to_move == Color::White { "w" } else { "b" })?;

        write!(f, " ")?;
        if self.castling.any() {
            if self.castling.white_kingside {
                write!(f, "K")?;
            }
            if self.castling.white_queenside {
                write!(f, "Q")?;
            }
            if self.castling.black_kingside {
                write!(f, "k")?;
            }
            if self.castling.black_queenside {
                write!(f, "q")?;
            }
        } else {
            write!(f, "-")?;
        }

        match self.en_passant {
            Some(sq) => write!(f, " {}", sq)?,
            None => write!(f, " -")?,
        }

        write!(f, " {} {}", self.halfmove_clock, self.fullmove_number)
    }
}
