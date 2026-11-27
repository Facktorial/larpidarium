//! Stable identifiers used to reference actions, people, items and rooms.

use std::fmt;

/// A stable, human-readable identifier such as `abbey-gate` or `brother-anselm`.
///
/// Rules: 1 to 64 characters, lowercase ASCII letters, digits and single hyphens,
/// starting with a letter and not ending with a hyphen. File names must equal the id,
/// so these rules also keep file names portable.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, serde::Deserialize)]
#[serde(try_from = "String")]
pub struct Id(String);

/// Why a string is not a valid [`Id`].
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum IdError {
    /// The string is empty.
    #[error("id is empty")]
    Empty,
    /// The string is longer than [`Id::MAX_LEN`].
    #[error("id is {len} characters long, the maximum is {max}", max = Id::MAX_LEN)]
    TooLong {
        /// Actual length in bytes.
        len: usize,
    },
    /// The first character is not a lowercase ASCII letter.
    #[error("id must start with a lowercase letter (a-z)")]
    BadStart,
    /// A character outside `a-z`, `0-9` and `-`.
    #[error("id contains {found:?}; use only a-z, 0-9 and '-'")]
    BadChar {
        /// The offending character.
        found: char,
    },
    /// Two hyphens in a row, or a hyphen at the end.
    #[error("id has a misplaced '-' (doubled or at the end)")]
    BadHyphen,
}

impl Id {
    /// Maximum length of an id in characters.
    pub const MAX_LEN: usize = 64;

    /// Checks `s` against the id rules and wraps it.
    ///
    /// # Errors
    ///
    /// Returns an [`IdError`] describing the first rule that `s` breaks.
    pub fn parse(s: &str) -> Result<Self, IdError> {
        if s.is_empty() {
            return Err(IdError::Empty);
        }
        if s.len() > Self::MAX_LEN {
            return Err(IdError::TooLong { len: s.len() });
        }
        if !s.starts_with(|c: char| c.is_ascii_lowercase()) {
            return Err(IdError::BadStart);
        }
        if let Some(found) = s
            .chars()
            .find(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-'))
        {
            return Err(IdError::BadChar { found });
        }
        if s.contains("--") || s.ends_with('-') {
            return Err(IdError::BadHyphen);
        }
        Ok(Self(s.to_owned()))
    }

    /// The id as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for Id {
    type Error = IdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::parse(&s)
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_typical_ids() {
        for s in ["abbey-gate", "a", "room-12", "brother-anselm"] {
            assert_eq!(Id::parse(s).unwrap().as_str(), s);
        }
    }

    #[test]
    fn rejects_each_rule() {
        assert_eq!(Id::parse(""), Err(IdError::Empty));
        assert_eq!(
            Id::parse(&"a".repeat(65)),
            Err(IdError::TooLong { len: 65 })
        );
        assert_eq!(Id::parse("1st"), Err(IdError::BadStart));
        assert_eq!(Id::parse("-a"), Err(IdError::BadStart));
        assert_eq!(Id::parse("Abbey"), Err(IdError::BadStart));
        assert_eq!(
            Id::parse("abbey gate"),
            Err(IdError::BadChar { found: ' ' })
        );
        assert_eq!(Id::parse("kříž"), Err(IdError::BadChar { found: 'ř' }));
        assert_eq!(Id::parse("a--b"), Err(IdError::BadHyphen));
        assert_eq!(Id::parse("a-"), Err(IdError::BadHyphen));
    }

    #[test]
    fn max_len_is_inclusive() {
        assert!(Id::parse(&"a".repeat(Id::MAX_LEN)).is_ok());
    }
}
