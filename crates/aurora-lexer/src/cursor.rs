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
    #[must_use]
    pub const fn offset(&self) -> usize {
        self.offset
    }

    /// Peeks at the next character without consuming it.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.chars.clone().next()
    }

    /// Peeks at the character after the next character (offset + 1).
    #[inline]
    pub fn peek_next(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next()?;
        iter.next()
    }

    /// Peeks at the character at offset + 2.
    #[inline]
    pub fn peek_third(&self) -> Option<char> {
        let mut iter = self.chars.clone();
        iter.next()?;
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

    /// Consumes the next character if it equals the expected char.
    #[inline]
    pub fn consume_if_eq(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    /// Consumes characters as long as the predicate returns true.
    pub fn eat_while(&mut self, mut predicate: impl FnMut(char) -> bool) {
        while let Some(c) = self.peek() {
            if predicate(c) {
                self.next_char();
            } else {
                break;
            }
        }
    }

    /// Slices a range of the source code by start and end byte offsets.
    #[inline]
    #[must_use]
    pub fn slice(&self, start: usize, end: usize) -> &'a str {
        let s = start.min(self.source.len());
        let e = end.min(self.source.len()).max(s);
        &self.source[s..e]
    }

    /// Checks if we reached end of file.
    #[inline]
    #[must_use]
    pub fn is_eof(&self) -> bool {
        self.offset >= self.source.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cursor_peeks_and_eats() {
        let mut cursor = Cursor::new("0x123abc");
        assert_eq!(cursor.peek(), Some('0'));
        assert_eq!(cursor.peek_next(), Some('x'));
        assert_eq!(cursor.peek_third(), Some('1'));

        assert_eq!(cursor.next_char(), Some('0'));
        assert_eq!(cursor.next_char(), Some('x'));

        cursor.eat_while(|c| c.is_ascii_hexdigit());
        assert_eq!(cursor.offset(), 8);
        assert!(cursor.is_eof());
    }
}
