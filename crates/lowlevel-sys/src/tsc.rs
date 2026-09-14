//! Hardware Time-Stamp Counter (TSC) profiling without OS system calls.

#[derive(Debug, Clone, Copy)]
pub struct HardwareTimer {
    start_cycles: u64,
}

impl HardwareTimer {
    #[inline(always)]
    pub fn start() -> Self {
        Self {
            start_cycles: read_tsc(),
        }
    }

    #[inline(always)]
    pub fn elapsed_cycles(&self) -> u64 {
        let current = read_tsc();
        current.saturating_sub(self.start_cycles)
    }

    /// Approximate elapsed microseconds assuming a base TSC frequency (e.g. 3.0 GHz)
    #[inline(always)]
    pub fn elapsed_approx_micros(&self, ghz: f64) -> f64 {
        let cycles = self.elapsed_cycles() as f64;
        let cycles_per_micro = ghz * 1000.0;
        cycles / cycles_per_micro
    }
}

#[inline(always)]
pub fn read_tsc() -> u64 {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::x86_64::_rdtsc()
    }
    #[cfg(target_arch = "aarch64")]
    unsafe {
        let mut val: u64;
        core::arch::asm!("mrs {}, cntvct_el0", out(reg) val);
        val
    }
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64
    }
}
