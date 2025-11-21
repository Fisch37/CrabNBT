use std::{iter::FusedIterator, num::NonZero};

use crate::nbt::error::SnbtDeserialisationError;

type Result<T> = std::result::Result<T, SnbtDeserialisationError>;

/// A more flexible replacement for `std::str::Chars`.
/// Allows character-by-character iteration over a string,
/// but with built-in support for peeking and teeing (via clone())
#[must_use]
pub struct StrVisitor<'a> {
    slice: &'a str,
    /// Position offset to the start of the slice (in bytes)
    position: usize,
}
impl<'a> StrVisitor<'a> {
    /// Creates a new visitor over the given slice, starting at position 0.
    pub fn new(slice: &'a str) -> Self {
        StrVisitor { slice, position: 0 }
    }

    /// Returns the next char in the visitor without advancing
    pub fn peek(&self) -> Option<char> {
        self.as_str().chars().next()
    }

    pub fn as_str(&self) -> &'a str {
        &self.slice[self.position..]
    }
}
impl<'a> Clone for StrVisitor<'a> {
    /// Creates another visitor, backed by the same slice at the current position
    fn clone(&self) -> Self {
        StrVisitor {
            slice: self.slice,
            position: self.position,
        }
    }
}
impl<'a> Iterator for StrVisitor<'a> {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        let result = self.peek();
        if result.is_some() {
            self.position = next_char_boundary(self.slice, self.position);
        }
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, upper) = self.slice.chars().size_hint();
        (lower - self.position, upper.map(|i| i - self.position))
    }
}
impl<'a> FusedIterator for StrVisitor<'a> {}

/// Finds the next char boundary greater than index. Saturates to slice.len()
///
/// This can be replaced once ceil_char_boundary is stabilised (requiring Rust 2025 Edition).
/// see rust-lang/rust#93743 for more
fn next_char_boundary(slice: &str, index: usize) -> usize {
    for i in (index + 1)..slice.len() {
        if slice.is_char_boundary(i) {
            return i;
        }
    }
    slice.len()
}

pub(crate) fn expect_char(chars: &mut dyn Iterator<Item = char>, expected: char) -> Result<()> {
    expect_condition(chars, &|c| c == expected, expected).map(|_| ())
}

pub(crate) fn expect_condition<S: ToString>(
    chars: &mut dyn Iterator<Item = char>,
    match_condition: &dyn Fn(char) -> bool,
    expected: S,
) -> Result<char> {
    match chars.next().ok_or(SnbtDeserialisationError::eof("{"))? {
        x if match_condition(x) => Ok(x),
        found => Err(SnbtDeserialisationError::unexpected(expected, found)),
    }
}

pub(crate) fn consume_while(visitor: &mut StrVisitor, condition: &dyn Fn(char) -> bool) {
    while let Some(future) = visitor.peek() {
        if !condition(future) {
            break;
        }
        visitor.next();
    }
}

pub(crate) fn consume_whitespace(visitor: &mut StrVisitor) {
    consume_while(visitor, &|c| c.is_whitespace())
}

pub(crate) fn read_tag_name(visitor: &mut StrVisitor) -> Result<String> {
    let mut result = String::new();
    let quote_char: Option<NonZero<char>>; // using NonZero as a small optimisation
    {
        let tag_start = visitor
            .next()
            .ok_or(SnbtDeserialisationError::eof("any character"))?;
        if tag_start == '"' || tag_start == '\'' {
            // SAFETY: We have just shown that the value cannot be zero.
            quote_char = Some(unsafe { NonZero::new_unchecked(tag_start) })
        } else {
            quote_char = None;
            result.push(tag_start);
        };
    }

    while quote_char.is_some() || visitor.peek().map(|c| c != ':').unwrap_or(false) {
        let c = match visitor.next() {
            Some(c) => c,
            None => return Err(SnbtDeserialisationError::eof("any tag character")),
        };
        if c == '\\' {
            result.push(parse_escape_sequence(visitor)?);
        } else if quote_char.map(|q| c == q.into()).unwrap_or(false) {
            break;
        } else {
            result.push(c);
        }
    }
    result.shrink_to_fit();
    Ok(result)
}

/// read and evaluate an SNBT escape sequence.
/// Assumes the \ character was already read.
fn parse_escape_sequence(visitor: &mut StrVisitor) -> Result<char> {
    let escaped_char = visitor
        .next()
        .ok_or(SnbtDeserialisationError::eof("any escapable character"))?;
    // see https://minecraft.wiki/w/NBT_format#Escape_sequences
    Ok(match escaped_char {
        '\\' => '\\',
        '\'' => '\'',
        '"' => '"',
        'b' => '\x08', // backspace
        'f' => '\x0C', // formfeed
        'n' => '\n',
        'r' => '\r',
        's' => ' ',
        't' => '\t',
        'x' => {
            todo!()
        }
        'u' => {
            todo!()
        }
        'U' => {
            todo!()
        }
        'N' => {
            todo!()
        }
        c => {
            return Err(SnbtDeserialisationError::unexpected(
                "any escapable character",
                c,
            ))
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::nbt::de_utils::{parse_escape_sequence, StrVisitor};

    #[test]
    fn escape_sequences() {
        fn parse_escape_with_str(slice: &str) -> char {
            parse_escape_sequence(&mut StrVisitor::new(slice)).unwrap()
        }
        assert_eq!(parse_escape_with_str("n"), '\n');
        assert_eq!(parse_escape_with_str("r"), '\r');
    }
}
