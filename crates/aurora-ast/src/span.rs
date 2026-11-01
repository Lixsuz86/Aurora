//! Source location and byte offset span representations.

use std::fmt;

/// Represents a source byte range in an input string [start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Span {
    /// Byte index where the span begins.
    pub start: usize,
    /// Byte index where the span ends (exclusive).
    pub end: usize,
}

impl Span {
    /// Creates a new span with explicit start and end byte offsets.
    #[inline]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    /// Creates an empty span at a specific byte offset.
    #[inline]
    pub const fn at(offset: usize) -> Self {
        Self {
            start: offset,
            end: offset,
        }
    }

    /// Combines two spans into a larger span encompassing both.
    #[inline]
    #[must_use]
    pub fn merge(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    /// Returns the length in bytes of the span.
    #[inline]
    pub const fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Returns true if the span covers zero bytes.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start, self.end)
    }
}

/// Human-readable 1-indexed line and column location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceLocation {
    pub line: usize,
    pub column: usize,
}

impl SourceLocation {
    pub const fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_span_merge() {
        let a = Span::new(2, 5);
        let b = Span::new(4, 10);
        let merged = a.merge(b);
        assert_eq!(merged.start, 2);
        assert_eq!(merged.end, 10);
        assert_eq!(merged.len(), 8);
    }
}
