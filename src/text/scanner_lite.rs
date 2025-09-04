use std::{marker::PhantomData, mem, slice, str};

/// A lightweight version of [`Scanner`](super::Scanner) that does not track its
/// current position in the source string.
///
/// This struct has no overhead over standard iteration techniques like
/// [`std::str::Chars`] and should be preferred when the tracking functionality
/// of `Scanner` is not needed.
#[derive(Clone, Copy, Debug)]
pub struct ScannerLite<'src> {
    pub(super) start: *const u8,
    pub(super) end: *const u8,
    pub(super) _data: PhantomData<&'src str>,
}

impl<'src> ScannerLite<'src> {
    #[inline]
    pub fn new(source_str: &'src str) -> Self {
        // SAFETY: just decomposing the slice into start and end pointers
        unsafe {
            Self {
                start: source_str.as_ptr(),
                end: source_str.as_ptr().add(source_str.len()),
                _data: PhantomData,
            }
        }
    }

    /// Skips a character if it is at the start of the remaining string.
    ///
    /// Returns `true` if the character was skipped.
    #[inline]
    pub fn expect_char(&mut self, expected: char) -> bool {
        if !self.remaining_str().starts_with(expected) {
            return false;
        }

        // SAFETY: we know the string starts with this char
        unsafe { self.expect_char_unchecked(expected) };

        true
    }

    /// Skips a known character without checking it.
    #[inline]
    pub unsafe fn expect_char_unchecked(&mut self, expected: char) {
        unsafe { self.skip_bytes_unchecked(expected.len_utf8()) };
    }

    /// Skips a string of characters if it is at the start of the remaining string.
    ///
    /// Returns `true` if the string was skipped.
    #[inline]
    pub fn expect_str(&mut self, expected: &str) -> bool {
        if !self.remaining_str().starts_with(expected) {
            return false;
        }

        // SAFETY: we have just verified the string starts with this prefix
        unsafe { self.expect_str_unchecked(expected) };

        true
    }

    /// Skips a known string of characters without checking it.
    #[inline]
    pub unsafe fn expect_str_unchecked(&mut self, expected: &str) {
        unsafe { self.skip_bytes_unchecked(expected.len()) };
    }

    /// Returns `true` if the remaining string is empty.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.start == self.end
    }

    /// Decodes the next [`char`] in the string and advances the scanner
    /// position.
    #[inline]
    pub fn next_char(&mut self) -> Option<char> {
        // SAFETY: fight me IRL
        unsafe { str::Chars::next(mem::transmute(self)) }
    }

    /// Advances the scanner to the start of the next line and returns the
    /// remainder of the current line.
    ///
    /// The newline character is included in the return value. If the scanner
    /// reaches the end of the source string before finding a newline, the
    /// entire remaining string is returned (which could be empty).
    #[inline]
    pub fn next_line(&mut self) -> &'src str {
        let start = self.start;

        while let Some(b) = self.peek_byte() {
            // SAFETY: we have just peeked a byte
            unsafe {
                self.skip_bytes_unchecked(1);
            }

            if b == b'\n' {
                break;
            }
        }

        unsafe {
            str::from_utf8_unchecked(slice::from_raw_parts(
                start,
                self.start as usize - start as usize,
            ))
        }
    }

    /// Returns the next byte in the source string without advancing the scanner
    /// position.
    ///
    /// Useful for validating ASCII characters without first decoding UTF-8
    /// codepoints into [`char`] values.
    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        if self.is_done() {
            return None;
        }

        // SAFETY: we know there's a valid byte here
        unsafe { Some(*self.start) }
    }

    /// Decodes the next [`char`] in the source string without advancing the
    /// scanner position.
    #[inline]
    pub fn peek_char(&self) -> Option<char> {
        self.clone().next_char()
    }

    /// Returns the length of the remaining string in bytes.
    #[inline]
    pub fn remaining_len(&self) -> usize {
        self.end as usize - self.start as usize
    }

    /// Returns a reference to the remaining string.
    #[inline]
    pub fn remaining_str(&self) -> &'src str {
        unsafe { str::from_utf8_unchecked(slice::from_raw_parts(self.start, self.remaining_len())) }
    }

    /// Skips zero or more ASCII whitespace characters.
    ///
    /// This is faster than `skip_whitespace` but does not recognize Unicode
    /// whitespace.
    #[inline]
    pub fn skip_ascii_whitespace(&mut self) {
        while let Some(byte) = self.peek_byte() {
            if !byte.is_ascii_whitespace() {
                break;
            }

            // SAFETY: we know self.start < self.end
            self.start = unsafe { self.start.add(1) };
        }
    }

    /// Skips the next `n` bytes in the string, without bounds checking or
    /// ensuring the scanner stops on a UTF-8 character boundary.
    #[inline]
    pub unsafe fn skip_bytes_unchecked(&mut self, n: usize) {
        unsafe { self.start = self.start.add(n) };
    }

    /// Skips the next character in the string.
    ///
    /// Returns `false` if the string was empty.
    #[inline]
    pub fn skip_char(&mut self) -> bool {
        if self.start == self.end {
            return false;
        }

        // SAFETY: string is verified non-empty
        unsafe {
            let first_byte = *self.start;

            let len_utf8 = match first_byte {
                ..128 => 1,
                // leading ones (two instructions on most CPUs) tells us the length of a
                // UTF-8 character
                _ => first_byte.leading_ones() as usize,
            };

            self.skip_bytes_unchecked(len_utf8);
        }

        true
    }

    /// Skips characters while they match a given predicate condition.
    #[inline]
    pub fn skip_chars_while(&mut self, predicate: impl Fn(char) -> bool) {
        while let Some(ch) = self.peek_char() {
            if !predicate(ch) {
                break;
            }

            // SAFETY: we know this char is at the start
            unsafe {
                self.start = self.start.add(ch.len_utf8());
            }
        }
    }

    /// Skips zero or more whitespace characters.
    pub fn skip_whitespace(&mut self) {
        self.skip_chars_while(char::is_whitespace);
    }
}
