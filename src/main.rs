use std::env;
use std::process;

use chess_notation::parse_san;

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: chess-notation <move> [<move> ...]");
        eprintln!("example: chess-notation e4 Nbxd7+ O-O exd8=Q#");
        process::exit(2);
    }

    let mut had_error = false;
    for arg in &args {
        match parse_san(arg) {
            Ok(mv) => println!("{}\t{}\t{:?}", arg, mv, mv),
            Err(e) => {
                eprintln!("{}: {}", arg, e);
                had_error = true;
            }
        }
    }

    if had_error {
        process::exit(1);
    }
}
