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
//! Move text (the actual game) is parsed separately; this module only
//! handles the bracketed header lines that come before it.

use crate::{err, ParseError};

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
