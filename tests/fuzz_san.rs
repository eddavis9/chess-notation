use chess_notation::parse_san;

// xorshift64 - deterministic and dependency-free, which is what matters here:
// a fixed seed means a failure reported by this test can always be reproduced
// by rerunning it, without pulling in a `rand` crate just to shuffle bytes.
struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn next_index(&mut self, bound: usize) -> usize {
        (self.next_u64() % bound as u64) as usize
    }
}

// Characters that show up in real SAN tokens, plus a couple of easy-to-confuse
// neighbors ('i', '9', '0'). Drawing from this alphabet instead of arbitrary
// bytes spends most of the fuzz budget near the boundary of valid input,
// where the parser's edge cases actually live.
const ALPHABET: &[char] = &[
    'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', '1', '2', '3', '4', '5', '6', '7', '8', '9', '0',
    'N', 'B', 'R', 'Q', 'K', 'O', 'x', '=', '+', '#', '-', ' ',
];

fn random_token(rng: &mut Rng) -> String {
    let len = rng.next_index(9);
    (0..len)
        .map(|_| ALPHABET[rng.next_index(ALPHABET.len())])
        .collect()
}

#[test]
fn never_panics_on_alphabet_soup() {
    let mut rng = Rng(0x9e3779b97f4a7c15);
    for _ in 0..100_000 {
        let token = random_token(&mut rng);
        // The only contract for garbage input is "return Err, don't panic" -
        // a bad slice index or arithmetic overflow here would unwind and
        // fail the test on its own.
        let _ = parse_san(&token);
    }
}

#[test]
fn never_panics_on_arbitrary_bytes() {
    // Beyond the SAN alphabet: raw bytes reinterpreted as UTF-8 (lossily,
    // since not every byte sequence is valid UTF-8), which is what actually
    // stresses char-vs-byte indexing on multi-byte sequences.
    let mut rng = Rng(0xd1b54a32d192ed03);
    for _ in 0..20_000 {
        let len = rng.next_index(12);
        let bytes: Vec<u8> = (0..len).map(|_| (rng.next_u64() % 256) as u8).collect();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let _ = parse_san(&text);
    }
}

#[test]
fn accepted_moves_round_trip_through_display() {
    let mut rng = Rng(0x2545f4914f6cdd1d);
    for _ in 0..100_000 {
        let token = random_token(&mut rng);
        if let Ok(mv) = parse_san(&token) {
            let printed = mv.to_string();
            let reparsed = parse_san(&printed).unwrap_or_else(|e| {
                panic!(
                    "printed form '{}' of accepted input '{}' failed to reparse: {}",
                    printed, token, e
                )
            });
            assert_eq!(
                mv, reparsed,
                "round trip mismatch: '{}' -> '{}' -> {:?}",
                token, printed, reparsed
            );
        }
    }
}
