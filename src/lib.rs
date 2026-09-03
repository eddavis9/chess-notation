//! Parsing and formatting for Standard Algebraic Notation (SAN), the move
//! notation used in almost all published chess games and PGN files.
//!
//! This crate only understands the *shape* of a move string (piece,
//! disambiguation, capture, destination, promotion, check marker). It does
//! not know about a board, so it cannot tell you whether a move is legal or
//! whether the disambiguation given actually resolves to a single piece.
//! That's a job for something that tracks position, and deliberately out of
//! scope here.

use std::fmt;

mod pgn;
pub use pgn::{parse_movetext, parse_tag_pairs, MoveText, MoveTextEntry, TagPair};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
}

impl PieceKind {
    fn from_letter(c: char) -> Option<PieceKind> {
        match c {
            'N' => Some(PieceKind::Knight),
            'B' => Some(PieceKind::Bishop),
            'R' => Some(PieceKind::Rook),
            'Q' => Some(PieceKind::Queen),
            'K' => Some(PieceKind::King),
            _ => None,
        }
    }

    // SAN never spells out the pawn letter, but FEN does. Keeping the
    // mapping total (rather than partial) means callers outside this
    // module never have to special-case Pawn.
    fn to_letter(&self) -> char {
        match self {
            PieceKind::Pawn => 'P',
            PieceKind::Knight => 'N',
            PieceKind::Bishop => 'B',
            PieceKind::Rook => 'R',
            PieceKind::Queen => 'Q',
            PieceKind::King => 'K',
        }
    }
}

/// A square on the board, zero-indexed from the a1 corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Square {
    pub file: u8, // 0 = a, 7 = h
    pub rank: u8, // 0 = rank 1, 7 = rank 8
}

impl Square {
    pub fn from_str(s: &str) -> Option<Square> {
        let bytes = s.as_bytes();
        if bytes.len() != 2 {
            return None;
        }
        let file = bytes[0];
        let rank = bytes[1];
        if !(b'a'..=b'h').contains(&file) || !(b'1'..=b'8').contains(&rank) {
            return None;
        }
        Some(Square {
            file: file - b'a',
            rank: rank - b'1',
        })
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}",
            (b'a' + self.file) as char,
            (b'1' + self.rank) as char
        )
    }
}

/// How the origin square of a piece move is narrowed down, when more than
/// one piece of that kind could otherwise reach the destination.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disambiguation {
    None,
    File(u8),
    Rank(u8),
    Square(Square),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckState {
    None,
    Check,
    Checkmate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Move {
    Castle {
        kingside: bool,
        check: CheckState,
    },
    Piece {
        piece: PieceKind,
        disambiguation: Disambiguation,
        capture: bool,
        to: Square,
        promotion: Option<PieceKind>,
        check: CheckState,
    },
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Move::Castle { kingside, check } => {
                write!(f, "{}", if *kingside { "O-O" } else { "O-O-O" })?;
                write_check(f, *check)
            }
            Move::Piece {
                piece,
                disambiguation,
                capture,
                to,
                promotion,
                check,
            } => {
                if *piece != PieceKind::Pawn {
                    write!(f, "{}", piece.to_letter())?;
                }
                match disambiguation {
                    Disambiguation::None => {}
                    Disambiguation::File(file) => write!(f, "{}", (b'a' + *file) as char)?,
                    Disambiguation::Rank(rank) => write!(f, "{}", (b'1' + *rank) as char)?,
                    Disambiguation::Square(sq) => write!(f, "{}", sq)?,
                }
                if *capture {
                    write!(f, "x")?;
                }
                write!(f, "{}", to)?;
                if let Some(p) = promotion {
                    write!(f, "={}", p.to_letter())?;
                }
                write_check(f, *check)
            }
        }
    }
}

fn write_check(f: &mut fmt::Formatter<'_>, check: CheckState) -> fmt::Result {
    match check {
        CheckState::None => Ok(()),
        CheckState::Check => write!(f, "+"),
        CheckState::Checkmate => write!(f, "#"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

fn err(msg: impl Into<String>) -> ParseError {
    ParseError {
        message: msg.into(),
    }
}

/// Parse a single SAN move, e.g. "e4", "Nbxd7+", "exd8=Q", "O-O-O#".
pub fn parse_san(input: &str) -> Result<Move, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(err("empty move"));
    }

    let (body, check) = if let Some(stripped) = trimmed.strip_suffix('#') {
        (stripped, CheckState::Checkmate)
    } else if let Some(stripped) = trimmed.strip_suffix('+') {
        (stripped, CheckState::Check)
    } else {
        (trimmed, CheckState::None)
    };

    if body.is_empty() {
        return Err(err("move has no body"));
    }

    // Some older sources write castling with the digit zero instead of the
    // letter O. Normalize before comparing.
    let normalized_castle = body.replace('0', "O");
    if normalized_castle == "O-O" {
        return Ok(Move::Castle {
            kingside: true,
            check,
        });
    }
    if normalized_castle == "O-O-O" {
        return Ok(Move::Castle {
            kingside: false,
            check,
        });
    }

    parse_piece_move(body, check)
}

fn parse_piece_move(body: &str, check: CheckState) -> Result<Move, ParseError> {
    let chars: Vec<char> = body.chars().collect();
    let mut i = 0;

    let piece = match chars.first() {
        Some(&c) if PieceKind::from_letter(c).is_some() => {
            i += 1;
            PieceKind::from_letter(c).unwrap()
        }
        _ => PieceKind::Pawn,
    };

    // Look for the promotion marker by char position, not byte position:
    // a disambiguation or file letter could in principle sit next to a
    // multi-byte character in garbage input, and byte offsets from
    // str::find would then land outside the char vector and panic on slice.
    let mut promotion = None;
    let mut end = chars.len();
    if let Some(eq_pos) = chars.iter().position(|&c| c == '=') {
        let promo_char = chars
            .get(eq_pos + 1)
            .copied()
            .ok_or_else(|| err("missing promotion piece"))?;
        let promo = PieceKind::from_letter(promo_char);
        if promo.is_none() || promo == Some(PieceKind::Pawn) {
            return Err(err(format!("invalid promotion piece '{}'", promo_char)));
        }
        promotion = promo;
        end = eq_pos;
    }

    if i > end {
        return Err(err(format!("malformed move '{}'", body)));
    }

    let core: Vec<char> = chars[i..end].to_vec();
    if core.is_empty() {
        return Err(err("move has no destination square"));
    }

    let capture = core.contains(&'x');
    let without_x: Vec<char> = core.into_iter().filter(|&c| c != 'x').collect();

    if without_x.len() < 2 {
        return Err(err(format!("cannot parse destination from '{}'", body)));
    }

    let dest_str: String = without_x[without_x.len() - 2..].iter().collect();
    let to = Square::from_str(&dest_str)
        .ok_or_else(|| err(format!("invalid destination square '{}'", dest_str)))?;

    let disambig_chars = &without_x[..without_x.len() - 2];
    let disambiguation = match disambig_chars.len() {
        0 => Disambiguation::None,
        1 => {
            let c = disambig_chars[0];
            if ('a'..='h').contains(&c) {
                Disambiguation::File(c as u8 - b'a')
            } else if ('1'..='8').contains(&c) {
                Disambiguation::Rank(c as u8 - b'1')
            } else {
                return Err(err(format!("invalid disambiguation '{}'", c)));
            }
        }
        2 => {
            let s: String = disambig_chars.iter().collect();
            Disambiguation::Square(
                Square::from_str(&s)
                    .ok_or_else(|| err(format!("invalid disambiguation square '{}'", s)))?,
            )
        }
        _ => {
            return Err(err(format!(
                "too many disambiguation characters in '{}'",
                body
            )))
        }
    };

    // A pawn capture always names its origin file (exd5, not just xd5) -
    // without it the move is genuinely ambiguous in plain text even though
    // our parser could technically produce a value for it.
    if capture && piece == PieceKind::Pawn && disambiguation == Disambiguation::None {
        return Err(err("pawn capture must specify the origin file"));
    }

    if promotion.is_some() && !(to.rank == 0 || to.rank == 7) {
        return Err(err("promotion is only legal on the last rank"));
    }

    Ok(Move::Piece {
        piece,
        disambiguation,
        capture,
        to,
        promotion,
        check,
    })
}
