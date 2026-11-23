use std::{char::CharTryFromError, num::{ParseFloatError, ParseIntError, TryFromIntError}};

use crate::nbt::snbt::de::{
    numbers::{NumberType, Radix, Signedness},
    utils::StrVisitor,
};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum SnbtDeserialisationError {
    #[error("Illegal combination of {0:?}, {1:?}, and {2:?}")]
    IllegalCombination(Radix, Signedness, NumberType),
    #[error(transparent)]
    ParseFloatError(#[from] ParseFloatError),
    #[error(transparent)]
    ParseIntError(#[from] ParseIntError),
    #[error(transparent)]
    TryFromIntError(#[from] TryFromIntError),
    // Note that this is different from [`std::char::TryFromCharError`]
    // CharTryFromError is used when conversion _to_ a char fails,
    // TryFromCharError is used when conversion _from_ a char fails.
    // If this is not confusing enough for your tastes,
    // I am certain Rust would welcome a PR to further obscure this difference.
    #[error(transparent)]
    CharTryFromError(#[from] CharTryFromError),
    #[error("UUID should contain at most 32 hexadecimal digits and 4 dashes")]
    UuidStringTooBig,
    #[error("UUID only has {0} of 4 dashes")]
    UuidNotEnoughDashes(u8),
    #[error("Expected {expected} at position {index}: {offending_area} <--[HERE]")]
    Unexpected {
        index: usize,
        offending_area: String,
        expected: &'static str,
    },
}
impl From<std::convert::Infallible> for SnbtDeserialisationError {
    fn from(_: std::convert::Infallible) -> Self {
        unreachable!()
    }
}

const MAX_OFFENSE_INFO_LENGTH: usize = 12;
impl SnbtDeserialisationError {
    pub fn from_visitor(visitor: &StrVisitor, expected: &'static str) -> Self {
        // get at most {MAX_OFFENSE_INFO_LENGTH} characters before the offending character
        let offending_area = visitor.get_slice()[..visitor.get_position()]
            .chars()
            .rev()
            .take(MAX_OFFENSE_INFO_LENGTH)
            // this extra allocation is sadly unavoidable without significant (and bug prone) effort
            .collect::<Vec<char>>()
            .iter()
            .rev()
            .collect();
        SnbtDeserialisationError::Unexpected {
            index: visitor.get_position(),
            offending_area,
            expected,
        }
    }
}
