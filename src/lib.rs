use js_sys::Number;
use js_sys::Uint8Array;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use web_sys::FileReaderSync;

thread_local! {
    static FILE_READER_SYNC: FileReaderSync = FileReaderSync::new().expect("Failed to create FileReaderSync. Help: make sure this is a web worker context.");
}

/// Wrapper around a `web_sys::File` that implements `Read` and `Seek`.
pub struct WebSysFile {
    file: web_sys::File,
    pos: u64,
}

impl WebSysFile {
    pub fn new(file: web_sys::File) -> Self {
        Self { file, pos: 0 }
    }

    /// File size in bytes.
    pub fn size(&self) -> u64 {
        let size_f64 = self.file.size();

        f64_to_u64_safe(size_f64).expect("file size is not a valid integer")
    }
}

/// Convert `u64` to `f64` but only if it can be done without loss of precision (if the number does
/// not exceed the `MAX_SAFE_INTEGER` constant).
fn u64_to_f64_safe(x: u64) -> Option<f64> {
    let x_float = x as f64;

    if x_float <= Number::MAX_SAFE_INTEGER {
        Some(x_float)
    } else {
        None
    }
}

/// Convert `f64` to `u64` but only if it can be done without loss of precision (if the number is
/// positive and it does not exceed the `MAX_SAFE_INTEGER` constant).
fn f64_to_u64_safe(x: f64) -> Option<u64> {
    if 0.0 <= x && x <= Number::MAX_SAFE_INTEGER {
        Some(x as u64)
    } else {
        None
    }
}

impl Read for WebSysFile {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, std::io::Error> {
        let buf_len = buf.len();
        let old_offset = self.pos;
        let offset_f64 = u64_to_f64_safe(old_offset).expect("offset too large");
        let offset_end_f64 = u64_to_f64_safe(
            old_offset.saturating_add(u64::try_from(buf_len).expect("buffer too large")),
        )
        .expect("offset + len too large");
        let blob = self
            .file
            .slice_with_f64_and_f64(offset_f64, offset_end_f64)
            .expect("failed to slice file");
        let array_buffer = FILE_READER_SYNC.with(|file_reader_sync| {
            file_reader_sync
                .read_as_array_buffer(&blob)
                .expect("failed to read as array buffer")
        });
        let array = Uint8Array::new(&array_buffer);
        let actual_read_bytes = array.byte_length();
        let actual_read_bytes_usize =
            usize::try_from(actual_read_bytes).expect("read too many bytes at once");
        // Copy to output buffer
        array.copy_to(&mut buf[..actual_read_bytes_usize]);
        // Update position
        self.pos = old_offset
            .checked_add(u64::from(actual_read_bytes))
            .expect("new position too large");

        Ok(actual_read_bytes_usize)
    }
}

// Copied these functions from std because they are unstable
fn overflowing_add_signed(lhs: u64, rhs: i64) -> (u64, bool) {
    let (res, overflowed) = lhs.overflowing_add(rhs as u64);
    (res, overflowed ^ (rhs < 0))
}

fn checked_add_signed(lhs: u64, rhs: i64) -> Option<u64> {
    let (a, b) = overflowing_add_signed(lhs, rhs);
    if b {
        None
    } else {
        Some(a)
    }
}

impl Seek for WebSysFile {
    fn seek(&mut self, style: SeekFrom) -> Result<u64, std::io::Error> {
        // Seek impl copied from std::io::Cursor
        let (base_pos, offset) = match style {
            SeekFrom::Start(n) => {
                self.pos = n;
                return Ok(n);
            }
            SeekFrom::End(n) => (self.size(), n),
            SeekFrom::Current(n) => (self.pos, n),
        };
        match checked_add_signed(base_pos, offset) {
            Some(n) => {
                self.pos = n;
                Ok(self.pos)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid seek to a negative or overflowing position",
            )),
        }
    }
}
