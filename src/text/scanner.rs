use std::slice;

use super::ScannerLite;

/// A specialized iterator designed for scanning and parsing strings.
///
/// This struct is an alternative to [`std::str::Chars`] that provides
/// additional utility functions and tracks the current line, column, and byte
/// number within the string. If position tracking is not needed,
/// [`ScannerLite`] provides similar utility with no overhead.
#[derive(Clone)]
pub struct Scanner<'src> {
    inner: ScannerLite<'src>,
    source_ptr: *const u8,
    line: usize,
    column: usize,
}

impl<'src> Scanner<'src> {
    #[inline]
    pub fn new(source_str: &'src str) -> Self {
        Self {
            inner: ScannerLite::new(source_str),
            source_ptr: source_str.as_ptr(),
            line: 0,
            column: 0,
        }
    }

    /// Gets the current column number.
    ///
    /// The column number starts at 1 and is based on the number of code points.
    /// For example, a tab character and a zero-width space character both have
    /// a column width of 1.
    #[inline]
    pub fn column(&self) -> usize {
        self.column
    }

    /// Skips an expected code point at the start of the remaining string.
    ///
    /// Returns `false` if the remaining string is empty or does not start with
    /// the expected [`char`] value.
    #[inline]
    pub fn expect_char(&mut self, expected: char) -> bool {
        if !self.inner.expect_char(expected) {
            return false;
        }

        self.record_char(expected);

        true
    }

    /// Skips an expected code point at the start of the remaining
    /// string.without checking it.
    ///
    /// # Safety
    ///
    /// The remaining string must start with the expected [`char`] value.
    #[inline]
    pub unsafe fn expect_char_unchecked(&mut self, ch: char) {
        unsafe {
            self.record_char(ch);
            self.inner.skip_bytes_unchecked(ch.len_utf8());
        }
    }

    /// Skips an expected prefix at the start of the remaining string.
    ///
    /// Returns `false` if the remaining string does not start with the expected
    /// string.
    #[inline]
    pub fn expect_str(&mut self, expected: &str) -> bool {
        if !self.remaining_str().starts_with(expected) {
            return false;
        }

        // SAFETY: we have checked the bounds of the remaining string
        unsafe {
            self.expect_str_unchecked(expected);
        }

        true
    }

    /// Skips an expected prefix at the start of the remaining string without
    /// checking it.
    ///
    /// # Safety
    ///
    /// The remaining string must start with the given [`str`] prefix.
    #[inline]
    pub unsafe fn expect_str_unchecked(&mut self, expected: &str) {
        unsafe {
            for b in expected.bytes() {
                // record UTF-8 start bytes as chars
                if !matches!(b, 128..192) {
                    self.record_char(b as char);
                }

                self.inner.skip_bytes_unchecked(1);
            }
        }
    }

    /// Returns `true` if the remaining string is empty.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.inner.is_done()
    }

    /// Gets the current line number.
    ///
    /// The line number starts at 1 and increases by one for each newline
    /// (`'\n'`) character scanned.
    #[inline]
    pub fn line(&self) -> usize {
        self.line
    }

    /// Advances past the next code point in the string and returns it as a
    /// [`char`].
    ///
    /// Returns [`None`] if the remining string is empty.
    #[inline]
    pub fn next_char(&mut self) -> Option<char> {
        let ch = self.inner.next_char()?;

        self.record_char(ch);

        Some(ch)
    }

    /// Advances past the next line in the string and returns it as a [`str`].
    ///
    ///  If the line ends with a newline character, it is included in the return
    /// value. Returns `""` if the remaining string is empty.
    #[inline]
    pub fn next_line(&mut self) -> &'src str {
        self.line += 1;
        self.column = 0;
        self.inner.next_line()
    }

    /// Returns the next byte in the string.
    ///
    /// Returns [`None`] if the remaining string is empty.
    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        self.inner.peek_byte()
    }

    /// Returns the next code point in the string as a [`char`].
    ///
    /// Returns [`None`] if the remaining string is empty.
    #[inline]
    pub fn peek_char(&self) -> Option<char> {
        self.inner.peek_char()
    }

    /// Gets the current position in the string.
    ///
    /// This is the byte offset from the start of the string.
    #[inline]
    pub fn position(&self) -> usize {
        unsafe { (self.inner.start as usize).unchecked_sub(self.source_ptr as usize) }
    }

    /// Returns a slice of the source string that ends at the current position.
    ///
    /// This function returns a reference to the part of the string that the
    /// scanner has already scanned.
    #[inline]
    pub fn preceding_str(&self) -> &'src str {
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.source_ptr, self.position())) }
    }

    /// Records a scanned code point by incrementing line and column numbers.
    #[inline]
    fn record_char(&mut self, value: char) {
        if value == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
    }

    /// Returns the length of the remaining string in bytes.
    #[inline]
    pub fn remaining_len(&self) -> usize {
        self.inner.remaining_len()
    }

    /// Returns a reference to the entire remaining string.
    #[inline]
    pub fn remaining_str(&self) -> &'src str {
        self.inner.remaining_str()
    }

    /// Skips any ASCII whitespace characters at the start of the remaining
    /// string.
    ///
    /// This is more efficient than `skip_whitespace` but does not recognize
    /// Unicode whitespace.
    #[inline]
    pub fn skip_ascii_whitespace(&mut self) {
        while let Some(byte) = self.inner.peek_byte() {
            if !byte.is_ascii_whitespace() {
                break;
            }

            self.record_char(byte as char);

            unsafe {
                self.inner.skip_bytes_unchecked(1);
            }
        }
    }

    /// Skips the next code point in the string.
    ///
    /// Returns `false` if the remaining string is empty.
    #[inline]
    pub fn skip_char(&mut self) -> bool {
        if self.is_done() {
            return false;
        }

        unsafe {
            let first_byte = *self.inner.start;

            // leading ones (two instructions on most CPUs) tells us the length of a
            // UTF-8 character
            let len_utf8 = match first_byte {
                ..128 => 1,
                _ => first_byte.leading_ones() as usize,
            };

            self.inner.skip_bytes_unchecked(len_utf8);
            self.record_char(first_byte as char);
        }

        true
    }

    /// Skips any code points at the start of the string that match a given
    /// condition.
    #[inline]
    pub fn skip_chars_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(ch) = self.peek_char() {
            if !predicate(ch) {
                break;
            }

            unsafe {
                self.expect_char_unchecked(ch);
            }
        }
    }

    /// Skips any whitespace characters at the start of the remaining string.
    #[inline]
    pub fn skip_whitespace(&mut self) {
        self.skip_chars_while(char::is_whitespace);
    }
}
