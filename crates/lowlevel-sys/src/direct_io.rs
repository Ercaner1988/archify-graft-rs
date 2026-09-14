//! Direct unbuffered disk I/O, bypassing OS page cache pollution.

use std::alloc::{alloc, dealloc, Layout};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::Path;

pub const SECTOR_ALIGNMENT: usize = 4096;

/// A buffer aligned to disk sector/page boundaries (4096 bytes) required for direct DMA.
pub struct AlignedBuffer {
    ptr: *mut u8,
    layout: Layout,
    capacity: usize,
}

unsafe impl Send for AlignedBuffer {}
unsafe impl Sync for AlignedBuffer {}

impl AlignedBuffer {
    pub fn new(capacity: usize) -> Self {
        let aligned_cap = (capacity + SECTOR_ALIGNMENT - 1) & !(SECTOR_ALIGNMENT - 1);
        let layout = Layout::from_size_align(aligned_cap, SECTOR_ALIGNMENT)
            .expect("Invalid layout for AlignedBuffer");
        let ptr = unsafe { alloc(layout) };
        if ptr.is_null() {
            panic!("Out of memory allocating aligned buffer of {} bytes", aligned_cap);
        }
        Self {
            ptr,
            layout,
            capacity: aligned_cap,
        }
    }

    #[inline(always)]
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.capacity) }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.ptr, self.capacity) }
    }

    #[inline(always)]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Drop for AlignedBuffer {
    fn drop(&mut self) {
        unsafe {
            dealloc(self.ptr, self.layout);
        }
    }
}

pub struct DirectReader;

impl DirectReader {
    /// Reads a file into an aligned buffer, using unbuffered direct DMA when beneficial.
    pub fn read_file<P: AsRef<Path>>(path: P) -> io::Result<Vec<u8>> {
        #[cfg(windows)]
        {
            if let Ok(data) = Self::read_file_win32_direct(path.as_ref()) {
                return Ok(data);
            }
        }

        Self::read_file_fallback(path.as_ref())
    }

    #[cfg(windows)]
    fn read_file_win32_direct(path: &Path) -> io::Result<Vec<u8>> {
        use std::os::windows::fs::OpenOptionsExt;

        let size = path.metadata()?.len() as usize;
        if size == 0 {
            return Ok(Vec::new());
        }

        // Windows unbuffered direct I/O requires read sizes to be sector-aligned (4096 bytes)
        const FILE_FLAG_NO_BUFFERING: u32 = 0x2000_0000;
        let aligned_size = (size + SECTOR_ALIGNMENT - 1) & !(SECTOR_ALIGNMENT - 1);
        let mut aligned = AlignedBuffer::new(aligned_size);

        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(FILE_FLAG_NO_BUFFERING)
            .open(path)?;

        let slice = aligned.as_mut_slice();
        let mut total = 0;
        while total < aligned_size {
            let n = file.read(&mut slice[total..])?;
            if n == 0 {
                break;
            }
            total += n;
        }

        Ok(aligned.as_slice()[..size.min(total)].to_vec())
    }

    fn read_file_fallback(path: &Path) -> io::Result<Vec<u8>> {
        let mut file = File::open(path)?;
        let size = file.metadata()?.len() as usize;

        if size < 64 * 1024 {
            let mut buf = Vec::with_capacity(size);
            file.read_to_end(&mut buf)?;
            return Ok(buf);
        }

        let mut aligned = AlignedBuffer::new(size);
        file.seek(SeekFrom::Start(0))?;

        let slice = aligned.as_mut_slice();
        let mut bytes_read = 0;
        while bytes_read < size {
            let n = file.read(&mut slice[bytes_read..size])?;
            if n == 0 {
                break;
            }
            bytes_read += n;
        }

        Ok(aligned.as_slice()[..bytes_read].to_vec())
    }
}
