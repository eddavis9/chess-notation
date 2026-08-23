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

This is a syntax parser, not a chess engine. It doesn't know about a board,
so it can't validate that a disambiguation is actually necessary, resolve
"the only knight that can legally move here", or reject a move that would
leave the mover's own king in check. See the issues for what's planned.
