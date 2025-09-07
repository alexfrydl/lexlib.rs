use std::{fmt, io, mem, ptr, str};

/// Reads UTF-8 data from an [`io::Read`] implementation character-by-character,
/// using a temporary storage buffer to minimize read calls.
///
/// # Example
///
/// ```no_run
/// let file = File::open("example.txt");
/// let mut buf = vec![0u8; 8192];
/// let mut reader = Utf8CharReader::new(&mut buf, file);
///
/// while let Some(ch) = reader.read_char()? {
///     print!("{}", ch);
/// }
/// ```
///
/// # Performance
///
/// This struct is designed for processing UTF-8 when the input is too large to
/// hold in memory or is of unknown length. Otherwise, it is usually more
/// performant to use [`io::Read::read_to_string`], or another method of reading
/// the entire input into memory, and iterate over the result with
/// [`str::Chars`].
pub struct Utf8CharReader<'buf, Inner> {
    reader: Utf8ChunkReader<'buf, Inner>,
    iter: str::Chars<'buf>,
}

/// Reads chunks of valid UTF-8 characters from an [`io::Read`] implementation,
/// using a temporary storage buffer to minimize read calls.
///
/// # Example
///
/// ```no_run
/// let file = File::open("example.txt");
/// let mut buf = vec![0u8; 8192];
/// let mut reader = Utf8ChunkReader::new(&mut buf, file);
///
/// while reader.read_chunk()? {
///     print!("{}", reader.chunk());
/// }
/// ```
///
/// # Performance
///
/// This struct is designed for processing UTF-8 when the input is too large to
/// hold in memory or is of unknown length. Otherwise, it is usually more
/// performant to use [`io::Read::read_to_string`] or another method of reading
/// the entire input into memory directly.
pub struct Utf8ChunkReader<'buf, Inner> {
    inner: Inner,
    buf: &'buf mut [u8],
    /// number of bytes in `buf`
    len: usize,
    /// number of bytes in `buf` that represent full, valid UTF-8 chars
    len_utf8: usize,
}

impl<'buf, Inner> Utf8CharReader<'buf, Inner>
where
    Inner: io::Read,
{
    #[inline]
    pub fn new(buf: &'buf mut [u8], inner: Inner) -> Self {
        Self {
            reader: Utf8ChunkReader::new(buf, inner),
            iter: "".chars(),
        }
    }

    /// Reads the next valid [`char`].
    ///
    /// Returns [`None`] if there is no data to read.
    pub fn read_char(&mut self) -> io::Result<Option<char>> {
        if let Some(ch) = self.iter.next() {
            return Ok(Some(ch));
        }

        let result = self.reader.read_chunk();

        unsafe {
            // fudging the lifetime is safe because this iter is always replaced
            // when we read a new chunk and is never exposed to calling code
            self.iter =
                mem::transmute::<str::Chars<'_>, str::Chars<'buf>>(self.reader.chunk().chars());

            Ok(match result? {
                // if `read_chunk` says the string is non-empty, we know there's
                // at least one `char` to get
                true => Some(self.iter.next().unwrap_unchecked()),
                false => None,
            })
        }
    }
}

impl<'buf, Inner> Utf8ChunkReader<'buf, Inner>
where
    Inner: io::Read,
{
    #[inline]
    pub fn new(buf: &'buf mut [u8], inner: Inner) -> Self {
        Self {
            inner,
            buf,
            len: 0,
            len_utf8: 0,
        }
    }

    /// Gets the last read chunk of valid UTF-8 characters.
    ///
    /// Returns `""` if no chunk has been read yet or an error has occured;
    /// otherwise, the return value is always a non-empty string.
    #[inline]
    pub fn chunk(&self) -> &str {
        unsafe { str::from_utf8_unchecked(self.buf.get_unchecked(..self.len_utf8)) }
    }

    /// Reads the next chunk of valid UTF-8 characters.
    ///
    /// Returns `false` if there is no data to read.
    pub fn read_chunk(&mut self) -> io::Result<bool> {
        unsafe {
            // reset the buffer

            let buf_ptr = self.buf.as_mut_ptr();
            let tail_ptr = buf_ptr.add(self.len_utf8);
            let tail_len = self.len - self.len_utf8;

            // copies any dangling invalid/incomplete UTF-8 chars to the front
            // of the buf
            ptr::copy(tail_ptr, buf_ptr, tail_len);

            self.len = tail_len;
            self.len_utf8 = 0;

            // read until the buffer is full

            while self.len != self.buf.len() {
                match self.inner.read(self.buf.get_unchecked_mut(self.len..)) {
                    Ok(0) => break,
                    Ok(n) => self.len += n,
                    Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                    Err(err) => return Err(err),
                }
            }

            if self.len == 0 {
                return Ok(false);
            }

            // validate utf8 bytes

            self.len_utf8 = self
                .buf
                // len is always > 0 and <= buf.len()
                .get_unchecked(..self.len)
                .utf8_chunks()
                .next()
                // utf8_chunks() always returns at least one element if the
                // slice is non-empty
                .unwrap_unchecked()
                .valid()
                .len();
        }

        if self.len_utf8 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            ));
        }

        Ok(true)
    }
}

impl<Inner> fmt::Debug for Utf8CharReader<'_, Inner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Utf8CharReader")
    }
}

impl<Inner> fmt::Debug for Utf8ChunkReader<'_, Inner> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Utf8ChunkReader")
    }
}
