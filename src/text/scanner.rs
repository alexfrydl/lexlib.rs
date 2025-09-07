use std::{fmt, slice, str};

/// A specialized iterator designed for scanning and parsing strings.
///
/// This struct is an alternative to [`str::Chars`] that offers additional
/// utility functions and tracks the current line, column, and byte position in
/// the string.
#[derive(Clone)]
pub struct Scanner<'src> {
    /// pointer to the start of the source string
    start: *const u8,
    /// pointer to the current position in the string
    head: *const u8,
    /// the value of the character at `head`, if any
    peek: Option<char>,
    /// the remaining string after the peeked char
    tail: str::Chars<'src>,
    line: usize,
    column: usize,
}

impl<'src> Scanner<'src> {
    pub fn new(source_str: &'src str) -> Self {
        let mut tail = source_str.chars();

        Self {
            start: source_str.as_ptr(),
            head: source_str.as_ptr(),
            peek: tail.next(),
            tail,
            line: 1,
            column: 1,
        }
    }

    /// Returns a pointer to the current position in the string.
    #[inline]
    pub fn as_ptr(&self) -> *const u8 {
        self.head
    }

    /// Gets the current column number.
    ///
    /// This is the number of code points since the beginning of the line,
    /// starting from 1.
    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    /// Consumes the next character in the string without checking that one
    /// exists.
    unsafe fn consume_char_unchecked(&mut self) {
        unsafe {
            if self.peek.unwrap_unchecked() == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }

            self.head = self.tail.as_str().as_ptr();
            self.peek = self.tail.next();
        }
    }

    /// Consumes the current line, including the newline character.
    fn consume_line(&mut self) {
        while let Some(ch) = self.take_char() {
            if ch == '\n' {
                return;
            }
        }
    }

    /// Consume characters in the string while they match a condition.
    fn consume_while(&mut self, mut condition: impl FnMut(char) -> bool) {
        unsafe {
            while let Some(ch) = self.peek
                && condition(ch)
            {
                self.consume_char_unchecked();
            }
        }
    }

    /// Consumes characters in the string until the next non-whitespace
    /// character.
    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    /// Gets the current line number.
    ///
    /// This is the number of newline characters scanned since the beginning of
    /// the string, starting from 1.
    #[inline]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Returns the [`char`] value of the next character in the string, without
    /// consuming it.
    ///
    /// Returns [`None`] if the remaining string is empty.
    #[inline]
    pub fn peek_char(&self) -> Option<char> {
        self.peek
    }

    /// Gets the current position in the string.
    ///
    /// This is the byte offset from the start of the string.
    #[inline]
    pub fn position(&self) -> usize {
        unsafe { (self.head as usize).unchecked_sub(self.start as usize) }
    }

    /// Returns a reference to the slice of the original source string that has
    /// already been scanned.
    ///
    /// The returned slice contains the entire preceding source string up to the
    /// current position.
    #[inline]
    pub fn preceding_str(&self) -> &'src str {
        unsafe { self.slice_back_unchecked(self.start) }
    }

    /// Returns the length of the remaining string in bytes.
    #[inline]
    pub fn remaining_len(&self) -> usize {
        let tail_str = self.tail.as_str();

        unsafe { tail_str.len() + (tail_str.as_ptr() as usize).unchecked_sub(self.head as usize) }
    }

    /// Returns a reference to the slice of the original source string that has
    /// not yet been scanned.
    ///
    /// The returned slice contains the entire remaining source string starting
    /// from the current position.
    pub fn remaining_str(&self) -> &'src str {
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.head, self.remaining_len())) }
    }

    /// Returns a slice of the source string that starts at a given pointer and
    /// ends at the current position.
    ///
    /// # Safety
    ///
    /// The given pointer must be inside the source string and before the
    /// current position, such as a value previously obtained from `as_ptr()`.
    #[inline]
    pub unsafe fn slice_back_unchecked(&self, from: *const u8) -> &'src str {
        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(
                from,
                (self.head as usize).unchecked_sub(from as usize),
            ))
        }
    }

    /// Consumes the next character in the string and returns its [`char`]
    /// value.
    ///
    /// Returns [`None`] if the remaining string is empty.
    #[inline]
    pub fn take_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;

        unsafe {
            self.consume_char_unchecked();
        }

        Some(ch)
    }

    /// Consumes the next character in the string and returns its [`char`] value
    /// if it satisfies a condition.
    ///
    /// Returns [`None`] if the remaining string is empty or starts with a
    /// character that does not satisfy the given `condition`.
    #[inline]
    pub fn take_char_if(&mut self, mut condition: impl FnMut(char) -> bool) -> Option<char> {
        if let Some(ch) = self.peek
            && condition(ch)
        {
            unsafe {
                self.consume_char_unchecked();
            }

            return Some(ch);
        }

        None
    }

    /// Consumes the next character in the string if it is equal to an expected
    /// [`char`] value.
    ///
    /// Returns `false` if the remaining string is empty or does not start with
    /// the expected character.
    #[inline]
    pub fn take_char_if_eq(&mut self, expected: char) -> bool {
        if self.peek != Some(expected) {
            return false;
        }

        unsafe {
            self.consume_char_unchecked();
        }

        true
    }

    /// Consumes the current line in the string and returns a reference to the
    /// slice that contains it.
    ///
    /// The newline character is included, if present. Returns `""` if the
    /// remaining string is empty.
    #[inline]
    pub fn take_line(&mut self) -> &'src str {
        let from = self.head;

        self.consume_line();

        unsafe { self.slice_back_unchecked(from) }
    }

    /// Consumes characters at the start of the remaining string that satisfy a
    /// condition and returns a reference to the slice that contains them.
    ///
    /// Returns `""` if the remaining string is empty or starts with a character
    /// that does not satisfy the given `condition`.
    #[inline]
    pub fn take_while(&mut self, predicate: impl FnMut(char) -> bool) -> &'src str {
        let from = self.head;

        self.consume_while(predicate);

        unsafe { self.slice_back_unchecked(from) }
    }

    /// Consumes whitespace characters at the start of the remaining string and
    /// returns a reference to the slice that contains them.
    ///
    /// Returns `""` if the remaining string is empty or starts with a
    /// non-whitespace character.
    #[inline]
    pub fn take_whitespace(&mut self) -> &'src str {
        let from = self.head;

        self.consume_whitespace();

        unsafe { self.slice_back_unchecked(from) }
    }
}

impl fmt::Debug for Scanner<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scanner")
            .field("column", &self.column)
            .field("line", &self.line)
            .field("peek_char", &self.peek)
            .field("position", &self.position())
            .field("remaining_len", &self.remaining_len())
            .finish()
    }
}
