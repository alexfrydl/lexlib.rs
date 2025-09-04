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
        unsafe {
            Self {
                start: source_str.as_ptr(),
                end: source_str.as_ptr().add(source_str.len()),
                _data: PhantomData,
            }
        }
    }

    /// Skips an expected code point at the start of the remaining string.
    ///
    /// Returns `false` if the remaining string is empty or does not start with
    /// the expected [`char`] value.
    #[inline]
    pub fn expect_char(&mut self, expected: char) -> bool {
        if !self.remaining_str().starts_with(expected) {
            return false;
        }

        unsafe {
            self.expect_char_unchecked(expected);
        }

        true
    }

    /// Skips an expected code point at the start of the remaining
    /// string.without checking it.
    ///
    /// # Safety
    ///
    /// The remaining string must start with the expected [`char`] value.
    #[inline]
    pub unsafe fn expect_char_unchecked(&mut self, expected: char) {
        unsafe { self.skip_bytes_unchecked(expected.len_utf8()) }
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
        unsafe { self.skip_bytes_unchecked(expected.len()) }
    }

    /// Returns `true` if the remaining string is empty.
    #[inline]
    pub fn is_done(&self) -> bool {
        self.start == self.end
    }

    /// Advances past the next code point in the string and returns it as a
    /// [`char`].
    ///
    /// Returns [`None`] if the remining string is empty.
    #[inline]
    pub fn next_char(&mut self) -> Option<char> {
        // ScannerLite has the same layout as str::Chars
        unsafe { str::Chars::next(mem::transmute::<&mut Self, &mut str::Chars>(self)) }
    }

    /// Advances past the next line in the string and returns it as a [`str`].
    ///
    ///  If the line ends with a newline character, it is included in the return
    /// value. Returns `""` if the remaining string is empty.
    #[inline]
    pub fn next_line(&mut self) -> &'src str {
        let start = self.start;

        while let Some(b) = self.peek_byte() {
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

    /// Returns the next byte in the string.
    ///
    /// Returns [`None`] if the remaining string is empty.
    #[inline]
    pub fn peek_byte(&self) -> Option<u8> {
        if self.is_done() {
            return None;
        }

        unsafe { Some(*self.start) }
    }

    /// Returns the next code point in the string as a [`char`].
    ///
    /// Returns [`None`] if the remaining string is empty.
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

    /// Skips any ASCII whitespace characters at the start of the remaining
    /// string.
    ///
    /// This is more efficient than `skip_whitespace` but does not recognize
    /// Unicode whitespace.
    #[inline]
    pub fn skip_ascii_whitespace(&mut self) {
        while let Some(byte) = self.peek_byte() {
            if !byte.is_ascii_whitespace() {
                break;
            }

            unsafe {
                self.start = self.start.add(1);
            }
        }
    }

    /// Skips the next `n` bytes in the string without checking them.
    ///
    /// # Safety
    ///
    /// The remaining string must start with a series of `n` bytes that does not
    /// end in the middle of a code point.
    #[inline]
    pub unsafe fn skip_bytes_unchecked(&mut self, n: usize) {
        unsafe {
            self.start = self.start.add(n);
        }
    }

    /// Skips the next code point in the string.
    ///
    /// Returns `false` if the remaining string is empty.
    #[inline]
    pub fn skip_char(&mut self) -> bool {
        if self.start == self.end {
            return false;
        }

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

    /// Skips any code points at the start of the string that match a given
    /// condition.
    #[inline]
    pub fn skip_chars_while(&mut self, condition: impl Fn(char) -> bool) {
        while let Some(ch) = self.peek_char() {
            if !condition(ch) {
                break;
            }

            unsafe {
                self.start = self.start.add(ch.len_utf8());
            }
        }
    }

    /// Skips any whitespace characters at the start of the remaining string.
    #[inline]
    pub fn skip_whitespace(&mut self) {
        self.skip_chars_while(char::is_whitespace);
    }
}
