# chess-notation

A small Rust library (plus a thin CLI) for parsing and formatting Standard
Algebraic Notation, the move notation you see in PGN files and published
games: `e4`, `Nbxd7+`, `exd8=Q`, `O-O-O#`.

## Why this exists

SAN looks simple until you actually have to parse it. A move string can
carry a piece letter, an optional disambiguating file/rank/square, a capture
marker, a destination square, a promotion, and a check or checkmate suffix -
and several of those pieces are optional in ways that depend on context:

- `Nbd7` and `N1d3` disambiguate the same way syntactically (one extra
  character) but mean different things depending on whether that character
  is a file letter or a rank digit.
- `Qh4xe1` needs a full square for disambiguation because neither the file
  nor the rank alone is unique.
- Pawn captures (`exd5`) always carry the origin file even though pawns
  never need disambiguation for a normal push - the `e` isn't decoration,
  it's required.
- Promotion (`e8=Q`) only makes sense on the back rank, and castling is
  written two different ways in the wild (`O-O` with the letter O, `0-0`
  with the digit zero).

This crate's job is narrow: turn a SAN token into a structured `Move` (or a
useful error), and format one back to text. It does not track a board, so
it can't tell you whether a move is legal, only whether it's well-formed.

## Library usage

```rust
use chess_notation::{parse_san, Move};

let mv = parse_san("Nbxd7+").unwrap();

match mv {
    Move::Piece { piece, disambiguation, capture, to, .. } => {
        println!("{:?} move, disambiguation {:?}, capture {}, lands on {}",
            piece, disambiguation, capture, to);
    }
    Move::Castle { kingside, .. } => {
        println!("castles {}", if kingside { "kingside" } else { "queenside" });
    }
}

// Move implements Display, so a parsed move can be turned back into text.
assert_eq!(mv.to_string(), "Nbxd7+");
```

`parse_tag_pairs` reads the bracketed header lines at the top of a PGN game
record - the Seven Tag Roster and any extra tags a source adds - and stops
at the first line that isn't a tag pair, which is where the move text
starts:

```rust
use chess_notation::parse_tag_pairs;

let pgn = "[Event \"F/S Return Match\"]\n[Site \"Belgrade, Serbia JUG\"]\n\n1. e4 e5 ...";
let tags = parse_tag_pairs(pgn).unwrap();
assert_eq!(tags[0].name, "Event");
assert_eq!(tags[0].value, "F/S Return Match");
```

`parse_movetext` reads the move list that follows the tag pairs: move numbers,
comments (`{...}` and `;...`), NAGs (`$1`), and `(...)` variations. Comments
and NAGs attach to the move they follow; a comment before the first move is
kept as a preamble instead:

```rust
use chess_notation::parse_movetext;

let body = "1. e4 e5 2. Nf3 {developing} Nc6 (2... d6 3. d4) 3. Bb5 a6 1-0";
let game = parse_movetext(body).unwrap();

assert_eq!(game.moves.len(), 6);
assert_eq!(game.moves[2].comment.as_deref(), Some("developing"));
assert_eq!(game.moves[3].variations.len(), 1);
assert_eq!(game.result.as_deref(), Some("1-0"));
```

`parse_fen` reads a Forsyth-Edwards Notation position record - piece
placement, side to move, castling rights, en passant target square, and the
two move counters - into a `Position`:

```rust
use chess_notation::{parse_fen, PieceKind, Square};

let pos = parse_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap();
let e4 = pos.piece_at(Square::from_str("e4").unwrap()).unwrap();
assert_eq!(e4.kind, PieceKind::Pawn);
assert_eq!(pos.en_passant, Some(Square::from_str("e3").unwrap()));
```

`Position` implements `Display` and formats back to the canonical FEN string.

## CLI usage

```
$ chess-notation e4 Nbxd7+ O-O exd8=Q#
e4      e4      Piece { piece: Pawn, disambiguation: None, capture: false, to: Square { file: 4, rank: 3 }, promotion: None, check: None }
Nbxd7+  Nbxd7+  Piece { piece: Knight, disambiguation: File(1), capture: true, to: Square { file: 3, rank: 6 }, promotion: None, check: Check }
O-O     O-O     Castle { kingside: true, check: None }
exd8=Q# exd8=Q# Piece { piece: Pawn, disambiguation: File(4), capture: true, to: Square { file: 3, rank: 7 }, promotion: Some(Queen), check: Checkmate }
```

Each line is the original token, its canonical re-formatted form, and the
parsed structure. A move that fails to parse is printed to stderr and the
process exits non-zero.

## Building

Standard library only, no dependencies:

```
cargo build
cargo test
```

## What's not here yet

This is a syntax parser, not a chess engine. `parse_fen` gives you a board
position, but nothing here yet resolves a SAN move against that position -
it can't validate that a disambiguation is actually necessary, resolve "the
only knight that can legally move here", or reject a move that would leave
the mover's own king in check. See the issues for what's planned.
