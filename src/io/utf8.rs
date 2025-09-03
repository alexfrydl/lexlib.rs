use std::{io, mem};

/// Reads one [`char`] at a time from an [`io::Read`] implementor, using a
/// temporary storage buffer to minimize read calls.
///
/// # Example
///
/// ```no_run
/// let file = io::File::open("example.txt");
/// let mut reader = Utf8CharReader::new(&mut vec![0u8; 8192], file);
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
/// [`std::str::Chars`].
pub struct Utf8CharReader<'buf, Inner> {
    reader: Utf8ChunkReader<'buf, Inner>,
    // fake static iterator for iterating over reader.chunk().chars()
    iter: std::str::Chars<'static>,
}

/// Reads chunks of valid UTF-8 characters from an [`io::Read`] implementor,
/// using a temporary storage buffer to minimize read calls.
///
/// # Example
///
/// ```no_run
/// let file = io::File::open("example.txt");
/// let mut reader = Utf8ChunkReader::new(&mut vec![0u8; 8192], file);
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
    pub fn new(buf: &'buf mut [u8], inner: Inner) -> Self {
        Self {
            reader: Utf8ChunkReader::new(buf, inner),
            iter: "".chars(),
        }
    }

    /// Reads the next valid [`char`].
    ///
    /// Returns [`None`] if there is no data to read.
    #[inline(always)]
    pub fn read_char(&mut self) -> io::Result<Option<char>> {
        match self.iter.next() {
            None => self.read_char_next_chunk(),
            ch => Ok(ch),
        }
    }

    /// Reads the next chunk and returns its first valid [`char`].
    ///
    /// This is a separate function because only the main `match` in `read_char`
    /// needs to be inlined.
    fn read_char_next_chunk(&mut self) -> io::Result<Option<char>> {
        let result = self.reader.read_chunk();

        // SAFETY: this iter is valid (even on error) until the next
        // call changes the chunk buffer.
        self.iter = unsafe { mem::transmute(self.reader.chunk().chars()) };

        result.map(|_| self.iter.next())
    }
}

impl<'buf, Inner> Utf8ChunkReader<'buf, Inner>
where
    Inner: io::Read,
{
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
    /// Returns `""` if no chunk has been read or an error has occured;
    /// otherwise, the return value is always a non-empty string.
    #[inline(always)]
    pub fn chunk(&self) -> &str {
        // SAFETY: the first `len_utf8` bytes of `buf` are always valid UTF-8,
        // verified in `read_chunk`.
        unsafe { str::from_utf8_unchecked(&self.buf[..self.len_utf8]) }
    }

    /// Reads the next chunk of valid UTF-8 characters.
    ///
    /// Returns `false` if there is no data to read.
    pub fn read_chunk(&mut self) -> io::Result<bool> {
        // reset the buffer

        self.buf.copy_within(self.len_utf8..self.len, 0);
        self.len -= self.len_utf8;
        self.len_utf8 = 0;

        // read until the buffer is full

        while self.len < self.buf.len() {
            match self.inner.read(&mut self.buf[self.len..]) {
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

        self.len_utf8 = match self.buf[..self.len].utf8_chunks().next() {
            Some(chunk) => chunk.valid().len(),
            None => 0,
        };

        if self.len_utf8 == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            ));
        }

        Ok(true)
    }
}
