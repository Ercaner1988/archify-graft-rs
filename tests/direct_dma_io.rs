//! Integration test: Direct DMA buffer and unbuffered disk I/O verification.

use lowlevel_sys::{cache_line_size, AlignedBuffer, DirectReader, SECTOR_ALIGNMENT};
use std::fs;
use std::io::Write;

#[test]
fn test_aligned_buffer_alignment_and_growth() {
    let buf = AlignedBuffer::new(100);
    // Buffer capacity must be a positive multiple of SECTOR_ALIGNMENT (4096)
    assert_eq!(buf.capacity(), SECTOR_ALIGNMENT);

    // Address must be 4096-byte aligned
    let ptr_addr = buf.as_slice().as_ptr() as usize;
    assert_eq!(
        ptr_addr % SECTOR_ALIGNMENT,
        0,
        "Buffer pointer is not sector aligned!"
    );

    let large_buf = AlignedBuffer::new(5000);
    assert_eq!(large_buf.capacity(), 8192);
    let large_ptr_addr = large_buf.as_slice().as_ptr() as usize;
    assert_eq!(large_ptr_addr % SECTOR_ALIGNMENT, 0);
}

#[test]
fn test_hardware_cache_detection() {
    let cl_size = cache_line_size();
    // Cache line size on modern x86/ARM is universally 64 or 128 bytes
    assert!(
        cl_size == 64 || cl_size == 128,
        "Unexpected cache line size: {}",
        cl_size
    );
}

#[test]
fn test_direct_reader_multi_size_files() {
    let temp_dir = std::env::temp_dir().join("archify_graft_dma_test");
    fs::create_dir_all(&temp_dir).unwrap();

    // 1. Empty file
    let empty_path = temp_dir.join("empty.txt");
    fs::write(&empty_path, b"").unwrap();
    let read_empty = DirectReader::read_file(&empty_path).unwrap();
    assert_eq!(read_empty.len(), 0);

    // 2. Small file (< 64KB)
    let small_path = temp_dir.join("small.txt");
    let small_content = b"pub fn test_small_function() { println!(\"DMA\"); }";
    fs::write(&small_path, small_content).unwrap();
    let read_small = DirectReader::read_file(&small_path).unwrap();
    assert_eq!(read_small, small_content);

    // 3. Multi-sector large file (> 8KB)
    let large_path = temp_dir.join("large.bin");
    let mut large_file = fs::File::create(&large_path).unwrap();
    let pattern = b"0123456789ABCDEF";
    for _ in 0..1024 {
        large_file.write_all(pattern).unwrap();
    }
    large_file.flush().unwrap();

    let read_large = DirectReader::read_file(&large_path).unwrap();
    assert_eq!(read_large.len(), 16 * 1024);
    assert_eq!(&read_large[0..16], pattern);
    assert_eq!(&read_large[16384 - 16..16384], pattern);

    let _ = fs::remove_dir_all(temp_dir);
}
