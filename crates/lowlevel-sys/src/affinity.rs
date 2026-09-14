//! Thread affinity to physical CPU cores, eliminating OS scheduler migration jitter.

#[cfg(windows)]
pub fn pin_current_thread_to_core(core_id: usize) -> Result<(), String> {
    use windows_sys::Win32::System::Threading::{GetCurrentThread, SetThreadAffinityMask};
    if core_id >= 64 {
        return Err("Core ID exceeds 64-bit affinity mask limit".to_string());
    }
    let mask: usize = 1 << core_id;
    unsafe {
        let handle = GetCurrentThread();
        let prev = SetThreadAffinityMask(handle, mask);
        if prev == 0 {
            Err("Failed to set thread affinity mask on Windows".to_string())
        } else {
            Ok(())
        }
    }
}

#[cfg(target_os = "linux")]
pub fn pin_current_thread_to_core(core_id: usize) -> Result<(), String> {
    unsafe {
        let mut cpuset: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_ZERO(&mut cpuset);
        libc::CPU_SET(core_id, &mut cpuset);
        let ret = libc::pthread_setaffinity_np(
            libc::pthread_self(),
            std::mem::size_of::<libc::cpu_set_t>(),
            &cpuset,
        );
        if ret != 0 {
            Err(format!(
                "pthread_setaffinity_np failed with error code: {}",
                ret
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(all(unix, not(target_os = "linux")))]
pub fn pin_current_thread_to_core(_core_id: usize) -> Result<(), String> {
    // macOS / BSD do not provide standard pthread_setaffinity_np
    Ok(())
}

#[cfg(not(any(windows, unix)))]
pub fn pin_current_thread_to_core(_core_id: usize) -> Result<(), String> {
    Ok(())
}
