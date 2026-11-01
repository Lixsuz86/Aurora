//! String cursor for efficient UTF-8 character scanning.

use std::str::Chars;

/// A lightweight character scanning cursor over a UTF-8 string slice.
#[derive(Debug, Clone)]
pub struct Cursor<'a> {
    source: &'a str,
    chars: Chars<'a>,
    offset: usize,
}

impl<'a> Cursor<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.chars(),
            offset: 0,
        }
    }

    /// Current byte offset into the source string.
    #[inline]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Peeks at the next character without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Peeks at the character after the next character.
    #[inline]
    pub fn peek_next(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next()?;
        iter.next()
    }

    /// Consumes and returns the next character.
    #[inline]
    pub fn next_char(&mut self) -> Option<char> {
        let c = self.chars.next()?;
        self.offset += c.len_utf8();
        Some(c)
    }

    /// Consumes the next character if it matches the predicate.
    #[inline]
    pub fn next_if(&mut self, predicate: impl FnOnce(char) -> bool) -> Option<char> {
        if self.peek().is_some_and(predicate) {
            self.next_char()
        } else {
            None
        }
    }

    /// Slices a range of the source code by start and end byte offsets.
    #[inline]
    pub fn slice(&self, start: usize, end: usize) -> &'a str {
        &self.source[start..end]
    }

    /// Checks if we reached end of file.
    #[inline]
    pub fn is_eof(&self) -> bool {
        self.offset >= self.source.len()
    }
}
