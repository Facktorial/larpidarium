//! Problems found while loading a game, with their location.

use std::ops::Range;

/// How bad a [`Diagnostic`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Probably a mistake, but the game can still be generated.
    Warning,
    /// The game cannot be generated until this is fixed.
    Error,
}

/// One problem in a game folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// Error or warning.
    pub severity: Severity,
    /// Path of the file, relative to the game folder, with `/` separators.
    pub file: String,
    /// Byte range in the file, when the problem has an exact location.
    pub span: Option<Range<usize>>,
    /// What is wrong.
    pub message: String,
    /// How to fix it, when there is an obvious fix.
    pub help: Option<String>,
}

impl Diagnostic {
    pub(crate) fn error(
        file: impl Into<String>,
        span: Option<Range<usize>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Error,
            file: file.into(),
            span,
            message: message.into(),
            help: None,
        }
    }

    pub(crate) fn warning(
        file: impl Into<String>,
        span: Option<Range<usize>>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            severity: Severity::Warning,
            ..Self::error(file, span, message)
        }
    }

    pub(crate) fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
