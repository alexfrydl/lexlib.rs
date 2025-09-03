use std::{iter::FusedIterator, mem, str::Chars};

/// A peekable iterator of [`char`] values.
///
/// This is a specialized alternative to using [`std::iter::Peekable`] with a
/// [`std::str::Chars`] iterator for better usability.
pub struct PeekableChars<'src> {
    peek: Option<char>,
    chars: Chars<'src>,
}

impl<'src> PeekableChars<'src> {
    #[inline]
    pub fn new(mut chars: Chars<'src>) -> Self {
        Self {
            peek: chars.next(),
            chars,
        }
    }

    /// Advances the iterator and returns the next [`char`].
    #[inline]
    pub fn next(&mut self) -> Option<char> {
        mem::replace(&mut self.peek, self.chars.next())
    }

    /// Advances the iterator and returns the next [`char`] if it matches a
    /// predicate.
    #[inline]
    pub fn next_if(&mut self, predicate: impl FnOnce(char) -> bool) -> Option<char> {
        match self.peek {
            Some(ch) if predicate(ch) => self.next(),
            _ => None,
        }
    }

    /// Advances the iterator and returns the next [`char`] if it is equal to an
    /// expected value.
    #[inline]
    pub fn next_if_eq(&mut self, expected: char) -> Option<char> {
        match self.peek {
            Some(ch) if ch == expected => self.next(),
            _ => None,
        }
    }

    /// Returns the next [`char`] without advancing the iterator.
    #[inline]
    pub fn peek(&self) -> Option<char> {
        self.peek
    }
}

impl<'src> Iterator for PeekableChars<'src> {
    type Item = char;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.next()
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self.peek {
            None => (0, Some(0)),
            _ => {
                let len = self.chars.as_str().len();

                return ((len + 7) / 4, Some(len + 1));
            }
        }
    }
}

impl FusedIterator for PeekableChars<'_> {}
