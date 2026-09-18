//! Hardware cache topology and memory alignment detection.

#[derive(Debug, Clone)]
pub struct HardwareTopology {
    pub logical_cores: usize,
    pub cache_line_bytes: usize,
}

impl HardwareTopology {
    pub fn detect() -> Self {
        let logical_cores = std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(4);
        let cache_line_bytes = cache_line_size();

        Self {
            logical_cores,
            cache_line_bytes,
        }
    }
}

/// Detects the CPU L1 data cache line size (standard is 64 bytes on modern x86_64 and aarch64).
pub fn cache_line_size() -> usize {
    #[cfg(target_arch = "x86_64")]
    {
        // CPUID leaf 1: EBX bits 15-8 contains cache line size in quadwords (CLFLUSH line size * 8)
        // SAFETY: CPUID leaf 1 is available on every x86_64 CPU.
        let cpuid = unsafe { core::arch::x86_64::__cpuid(1) };
        let clflush = ((cpuid.ebx >> 8) & 0xFF) as usize;
        if clflush > 0 {
            return clflush * 8;
        }
    }
    64 // Default fallback for modern CPUs
}
