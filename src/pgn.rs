//! Parsing for the PGN tag pair section (the "Seven Tag Roster" header
//! block that precedes the move text in a PGN game record), e.g.:
//!
//! ```text
//! [Event "F/S Return Match"]
//! [Site "Belgrade, Serbia JUG"]
//! [Date "1992.11.04"]
//! [Round "29"]
//! [White "Fischer, Robert J."]
//! [Black "Spassky, Boris V."]
//! [Result "1/2-1/2"]
//! ```
//!
//! `parse_movetext` handles the game body that follows: move numbers,
//! SAN moves, `{...}` comments, `;...` end-of-line comments, `$n` NAGs, and
//! `(...)` variations, e.g.:
//!
//! ```text
//! 1. e4 e5 2. Nf3 {developing} Nc6 (2... d6 3. d4) 3. Bb5 a6 1-0
//! ```

use crate::{err, Move, ParseError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagPair {
    pub name: String,
    pub value: String,
}

/// Parse the leading run of `[Name "value"]` header lines from a PGN game
/// record. Stops at the first blank line or line that isn't a tag pair,
/// which is where the move text begins - it is not an error for a game to
/// have move text after the tags, we just don't parse it here.
pub fn parse_tag_pairs(input: &str) -> Result<Vec<TagPair>, ParseError> {
    let mut tags = Vec::new();
    for line in input.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('[') {
            break;
        }
        tags.push(parse_tag_line(line)?);
    }
    Ok(tags)
}

fn parse_tag_line(line: &str) -> Result<TagPair, ParseError> {
    let inner = line
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .ok_or_else(|| err(format!("malformed tag pair '{}'", line)))?;

    let quote_start = inner
        .find('"')
        .ok_or_else(|| err(format!("tag pair missing quoted value: '{}'", line)))?;

    let name = inner[..quote_start].trim();
    if name.is_empty() {
        return Err(err(format!("tag pair missing name: '{}'", line)));
    }
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(err(format!("invalid tag name '{}'", name)));
    }

    let after_quote = &inner[quote_start + 1..];
    let closing = find_unescaped_quote(after_quote)
        .ok_or_else(|| err(format!("unterminated tag value in '{}'", line)))?;

    if !after_quote[closing + 1..].trim().is_empty() {
        return Err(err(format!("trailing content after tag value in '{}'", line)));
    }

    Ok(TagPair {
        name: name.to_string(),
        value: unescape_tag_value(&after_quote[..closing]),
    })
}

// A quote counts as the closing one unless it was escaped with a backslash.
// Walking char by char (rather than str::find) lets us skip the character
// right after a backslash so an escaped quote never terminates the value.
fn find_unescaped_quote(s: &str) -> Option<usize> {
    let mut chars = s.char_indices();
    while let Some((idx, c)) = chars.next() {
        if c == '\\' {
            chars.next();
        } else if c == '"' {
            return Some(idx);
        }
    }
    None
}

fn unescape_tag_value(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            result.push(chars.next().unwrap_or('\\'));
        } else {
            result.push(c);
        }
    }
    result
}

/// One parsed move plus the annotations PGN allows to trail it: a comment,
/// any Numeric Annotation Glyphs, and any variations (alternatives to this
/// move, each itself a move sequence).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoveTextEntry {
    pub mv: Move,
    pub comment: Option<String>,
    pub nags: Vec<u32>,
    pub variations: Vec<Vec<MoveTextEntry>>,
}

/// The parsed move-text body of a PGN game: the moves themselves, an
/// optional comment that precedes the first move (a preamble rather than
/// an annotation of any particular move), and the game termination marker
/// ("1-0", "0-1", "1/2-1/2", or "*"), if the input reached one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MoveText {
    pub preamble_comment: Option<String>,
    pub moves: Vec<MoveTextEntry>,
    pub result: Option<String>,
}

/// Parse the move-text body of a PGN game record - everything after the
/// tag pair header. Move numbers ("1.", "12...") are recognized and
/// discarded, comments and NAGs attach to the move they immediately
/// follow, and `(...)` variations attach to the move they're an
/// alternative to.
pub fn parse_movetext(input: &str) -> Result<MoveText, ParseError> {
    let mut cursor = Cursor::new(input);
    let seq = parse_sequence(&mut cursor, 0)?;
    Ok(MoveText {
        preamble_comment: seq.preamble,
        moves: seq.entries,
        result: seq.result,
    })
}

struct Sequence {
    entries: Vec<MoveTextEntry>,
    preamble: Option<String>,
    result: Option<String>,
}

// depth tracks variation nesting so an unmatched ')' at top level can be
// reported as an error rather than silently accepted as end of input.
fn parse_sequence(cur: &mut Cursor, depth: usize) -> Result<Sequence, ParseError> {
    let mut entries: Vec<MoveTextEntry> = Vec::new();
    let mut preamble: Option<String> = None;
    let mut result = None;

    loop {
        cur.skip_whitespace();
        match cur.peek() {
            None => break,
            Some(')') => {
                if depth == 0 {
                    return Err(err("unmatched ')' in move text"));
                }
                break;
            }
            Some('(') => {
                cur.bump();
                let sub = parse_sequence(cur, depth + 1)?;
                cur.skip_whitespace();
                if cur.peek() != Some(')') {
                    return Err(err("unterminated variation, missing ')'"));
                }
                cur.bump();
                let last = entries
                    .last_mut()
                    .ok_or_else(|| err("variation must follow a move"))?;
                last.variations.push(sub.entries);
            }
            Some('{') => {
                let comment = parse_comment(cur)?;
                match entries.last_mut() {
                    Some(entry) => append_comment(&mut entry.comment, comment),
                    None => append_comment(&mut preamble, comment),
                }
            }
            Some(';') => {
                let comment = read_line_comment(cur);
                match entries.last_mut() {
                    Some(entry) => append_comment(&mut entry.comment, comment),
                    None => append_comment(&mut preamble, comment),
                }
            }
            Some('$') => {
                let nag = parse_nag(cur)?;
                let last = entries
                    .last_mut()
                    .ok_or_else(|| err("NAG must follow a move"))?;
                last.nags.push(nag);
            }
            Some(c) if c.is_ascii_digit() => match read_digit_token(cur)? {
                DigitToken::MoveNumber => {}
                DigitToken::Result(token) => {
                    result = Some(token);
                    break;
                }
            },
            Some(_) => {
                let token = read_token(cur).to_string();
                if is_result_token(&token) {
                    result = Some(token);
                    break;
                }
                let mv = crate::parse_san(&token)?;
                entries.push(MoveTextEntry {
                    mv,
                    comment: None,
                    nags: Vec::new(),
                    variations: Vec::new(),
                });
            }
        }
    }

    Ok(Sequence {
        entries,
        preamble,
        result,
    })
}

fn append_comment(slot: &mut Option<String>, comment: String) {
    match slot {
        Some(existing) => {
            existing.push(' ');
            existing.push_str(&comment);
        }
        None => *slot = Some(comment),
    }
}

fn is_delimiter(c: char) -> bool {
    "(){};$".contains(c)
}

fn is_result_token(token: &str) -> bool {
    matches!(token, "1-0" | "0-1" | "1/2-1/2" | "*")
}

enum DigitToken {
    MoveNumber,
    Result(String),
}

// A move number is digits followed by dots ("1." or the "10..." form PGN
// uses to reintroduce a move number after a comment breaks the line). A
// digit can also start a result token ("1-0", "1/2-1/2"), which is the one
// case a SAN move token never produces, so the two can't be confused by
// their first character alone - we have to read past it to tell them apart.
fn read_digit_token(cur: &mut Cursor) -> Result<DigitToken, ParseError> {
    let start = cur.pos;
    while matches!(cur.peek(), Some(c) if c.is_ascii_digit()) {
        cur.bump();
    }
    match cur.peek() {
        Some('-') | Some('/') => {
            while matches!(cur.peek(), Some(c) if !c.is_whitespace() && !is_delimiter(c)) {
                cur.bump();
            }
            let token = &cur.input[start..cur.pos];
            if is_result_token(token) {
                Ok(DigitToken::Result(token.to_string()))
            } else {
                Err(err(format!("unrecognized token '{}'", token)))
            }
        }
        _ => {
            while cur.peek() == Some('.') {
                cur.bump();
            }
            Ok(DigitToken::MoveNumber)
        }
    }
}

fn read_token<'a>(cur: &mut Cursor<'a>) -> &'a str {
    let start = cur.pos;
    while matches!(cur.peek(), Some(c) if !c.is_whitespace() && !is_delimiter(c)) {
        cur.bump();
    }
    &cur.input[start..cur.pos]
}

fn parse_comment(cur: &mut Cursor) -> Result<String, ParseError> {
    cur.bump(); // consume '{'
    let start = cur.pos;
    match cur.rest().find('}') {
        Some(rel_end) => {
            let text = cur.input[start..start + rel_end].trim().to_string();
            cur.pos = start + rel_end + 1;
            Ok(text)
        }
        None => Err(err("unterminated comment, missing '}'")),
    }
}

fn read_line_comment(cur: &mut Cursor) -> String {
    cur.bump(); // consume ';'
    let start = cur.pos;
    while matches!(cur.peek(), Some(c) if c != '\n') {
        cur.bump();
    }
    cur.input[start..cur.pos].trim().to_string()
}

fn parse_nag(cur: &mut Cursor) -> Result<u32, ParseError> {
    cur.bump(); // consume '$'
    let start = cur.pos;
    while matches!(cur.peek(), Some(c) if c.is_ascii_digit()) {
        cur.bump();
    }
    let digits = &cur.input[start..cur.pos];
    if digits.is_empty() {
        return Err(err("'$' NAG marker must be followed by a number"));
    }
    digits
        .parse::<u32>()
        .map_err(|_| err(format!("invalid NAG '${}'", digits)))
}

// Byte-offset cursor over the input. '{', '}', '(', ')', ';', '$' and the
// ASCII digits/dots that make up move numbers and results are all
// single-byte, so slicing at the offsets found around them always lands on
// a char boundary even though comment text in between may be arbitrary
// UTF-8.
struct Cursor<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn new(input: &'a str) -> Self {
        Cursor { input, pos: 0 }
    }

    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn peek(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += c.len_utf8();
        Some(c)
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }
}
