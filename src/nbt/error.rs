use std::fmt::Display;

#[derive(Debug)]
pub struct SnbtDeserialisationError {
    pub expected: String,
    pub found: Option<char>,
}

impl SnbtDeserialisationError {
    fn new<S: ToString>(expected: S, found: Option<char>) -> Self {
        SnbtDeserialisationError {
            expected: expected.to_string(),
            found,
        }
    }

    pub fn eof<S: ToString>(expected: S) -> Self {
        Self::new(expected, None)
    }

    pub fn unexpected<S: ToString>(expected: S, found: char) -> Self {
        Self::new(expected, Some(found))
    }
}

impl Display for SnbtDeserialisationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Expected {}, found {}",
            self.expected,
            self.found
                .map(|char| char.into())
                .unwrap_or("EOF".to_string())
        )
    }
}
