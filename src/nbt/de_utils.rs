use std::iter::FusedIterator;

use crate::nbt::error::SnbtDeserialisationError;

pub type Result<T> = std::result::Result<T, SnbtDeserialisationError>;

/// A middleman trait for [`std::str::FromStr`].
/// 
pub trait FromVisitor: Sized {
    type Err;

    fn from_visitor(visitor: &mut StrVisitor) -> std::result::Result<Self, Self::Err>;
}

#[macro_export]
/// Provides an implementation of [`std::str::FromStr`] for types that implement [`FromVisitor`].
/// It calls [`FromVisitor::from_visitor`] and if, after the method returns,
/// the visitor is not exhausted, returns an [`Err`] variant (otherwise, returns the result)
macro_rules! impl_FromStr_through_FromVisitor {
    ($type:ty) => {
        impl std::str::FromStr for $type {
            type Err = <$type as FromVisitor>::Err;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                let mut visitor = StrVisitor::new(s);
                Self::from_visitor(&mut visitor)
                    .and_then(|res| {
                        match visitor.peek() {
                            None => Ok(res),
                            Some(c) => Err(SnbtDeserialisationError::unexpected("EOF", c))
                        }
                    })
            }
        }
    };
}

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

    /// Returns the next char in the visitor without advancing.
    /// 
    /// It is guaranteed that the result of this method 
    /// will be the same as the next result of [`Self::next`]
    pub fn peek(&self) -> Option<char> {
        self.as_str().chars().next()
    }

    pub fn previous(&mut self) -> Option<char> {
        self.position = previous_char_boundary(self.slice, self.position);
        self.peek()
    }

    pub fn next_if<P: FnOnce(char) -> bool>(&mut self, predicate: P) -> Option<char> {
        self.peek().filter(|c| predicate(*c)).inspect(|_| { self.next(); })
    }

    pub fn as_str(&self) -> &'a str {
        &self.slice[self.position..]
    }

    pub fn get_position(&self) -> usize {
        return self.position
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
/// This can be replaced once [`str::ceil_char_boundary`] is stabilised 
/// (requiring Rust 2025 Edition).
/// see rust-lang/rust#93743 for more
fn next_char_boundary(slice: &str, index: usize) -> usize {
    for i in (index + 1)..slice.len() {
        if slice.is_char_boundary(i) {
            return i;
        }
    }
    slice.len()
}

/// Finds the last char boundary less than index. Saturates to 0
/// 
/// Like [`next_char_boundary`] this can be replaced
/// once [`str::floor_char_boundary`] is stabilised.
fn previous_char_boundary(slice: &str, index: usize) -> usize {
    for i in (0..index).rev() {
        if slice.is_char_boundary(i) {
            return i;
        }
    }
    0
}

pub(crate) fn expect_char(chars: &mut dyn Iterator<Item = char>, expected: char) -> Result<()> {
    expect_condition(chars, &|c| c == expected, expected).map(|_| ())
}

pub(crate) fn expect_condition<S: ToString, P: FnOnce(char) -> bool>(
    chars: &mut dyn Iterator<Item = char>,
    predicate: P,
    expected: S,
) -> Result<char> {
    match chars.next().ok_or(SnbtDeserialisationError::eof("{"))? {
        x if predicate(x) => Ok(x),
        found => Err(SnbtDeserialisationError::unexpected(expected, found)),
    }
}

/// Expect a literal series of characters in `chars`, matching `expected`.
/// 
/// If the iterators don't match,
/// returns an Err variant with [`SnbtDeserialisationError::unexpected`] and `chars` and `expected`
/// will be positioned _after_ the mismatched characters.
/// 
/// If `chars` is shorter than `expected`, returns an Err with [`SnbtDeserialisationError::eof`].
pub(crate) fn expect_literal(
    chars: &mut dyn Iterator<Item = char>,
    expected: &mut dyn Iterator<Item = char>
) -> Result<()> {
    for expected_char in expected {
        let current_char = chars.next()
            .ok_or(SnbtDeserialisationError::eof(expected_char))?;
        if expected_char != current_char {
            return Err(SnbtDeserialisationError::unexpected(expected_char, current_char));
        }
    }
    Ok(())
}

pub(crate) fn expect_string(
    chars: &mut dyn Iterator<Item = char>,
    expected: &str
) -> Result<()> {
    expect_literal(chars, &mut expected.chars())
}

pub(crate) fn consume_while<P>(visitor: &mut StrVisitor, mut condition: P)
    where P: FnMut(char) -> bool
{
    while let Some(future) = visitor.peek() {
        if !condition(future) {
            break;
        }
        visitor.next();
    }
}

pub(crate) fn consume_whitespace(visitor: &mut StrVisitor) {
    consume_while(visitor, |c| c.is_whitespace())
}

pub(crate) fn read_slice_while<'a, P>(visitor: &mut StrVisitor<'a>, mut predicate: P) -> &'a str
    where P: FnMut(char) -> bool
{
    let start_position = visitor.get_position();
    let slice = visitor.as_str();
    while visitor.next_if(&mut predicate).is_some() { }
    &slice[..(visitor.get_position() - start_position)]
}

pub(crate) fn read_string(visitor: &mut StrVisitor) -> Result<String> {
    let first_char = visitor.peek()
        .ok_or(SnbtDeserialisationError::eof("quote or any tag character"))?;
    match first_char {
        '"' | '\'' => read_quoted_string(visitor),
        _ => read_unquoted_string(visitor)
    }
}

pub(crate) fn read_quoted_string(visitor: &mut StrVisitor) -> Result<String> {
    let mut result = String::new();
    let quote_char = visitor.next()
        .ok_or(SnbtDeserialisationError::eof("one of \" or '"))?;
    if quote_char != '"' || quote_char != '\'' {
        return Err(SnbtDeserialisationError::unexpected("either \" or '", quote_char));
    }

    while let Some(c) = visitor.next() {
        if c == quote_char {
            result.shrink_to_fit();
            return Ok(result);
        } else if c == '\\' {
            result.push(parse_escape_sequence(visitor)?);
        } else {
            result.push(c);
        }
    }
    Err(SnbtDeserialisationError::eof("any string character"))
}

/// Reads an unquoted SNBT string value. 
/// Returns an [`Err`] variant if the first character doesn't match, 
/// placing `visitor` over that character.
/// 
/// This algorithm is greedy, meaning it will consume all valid characters in a sequence.
pub(crate) fn read_unquoted_string(visitor: &mut StrVisitor) -> Result<String> {
    const START_EXPECTED: &'static str = "one of [a-zA-z]|-|\\+|\\.";
    match visitor.peek().ok_or(SnbtDeserialisationError::eof(START_EXPECTED))? {
        c if !char_may_start_unquoted(c) => 
            return Err(SnbtDeserialisationError::unexpected(START_EXPECTED, c)),
        _ => { }
    }
    
    let mut result = String::new();
    while let Some(c) = visitor.next_if(char_may_be_unquoted) {
        // escape sequences are not allowed in unquoted strings
        result.push(c)
    }
    Ok(result)
}

pub(crate) fn char_may_be_unquoted(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '+' || c == '.'
}
pub(crate) fn char_may_start_unquoted(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
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
            todo!("\\x")
        }
        'u' => {
            todo!("\\u")
        }
        'U' => {
            todo!("\\U")
        }
        'N' => {
            todo!("\\N")
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
