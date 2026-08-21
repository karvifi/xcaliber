// SPDX-License-Identifier: Apache-2.0
//! Small, dependency-free process metrics used by the CLI diagnostics.

/// Returns the process peak resident set size in bytes.
///
/// A zero result means that the operating system did not provide the metric.
#[cfg(unix)]
pub fn peak_rss_bytes() -> u64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    if unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) } != 0 {
        return 0;
    }

    #[cfg(target_os = "macos")]
    {
        usage.ru_maxrss as u64
    }
    #[cfg(not(target_os = "macos"))]
    {
        (usage.ru_maxrss as u64).saturating_mul(1024)
    }
}

#[cfg(windows)]
#[repr(C)]
struct ProcessMemoryCounters {
    cb: u32,
    page_fault_count: u32,
    peak_working_set_size: usize,
    working_set_size: usize,
    quota_peak_paged_pool_usage: usize,
    quota_paged_pool_usage: usize,
    quota_peak_non_paged_pool_usage: usize,
    quota_non_paged_pool_usage: usize,
    pagefile_usage: usize,
    peak_pagefile_usage: usize,
}

#[cfg(windows)]
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> isize;
}

#[cfg(windows)]
#[link(name = "psapi")]
extern "system" {
    fn GetProcessMemoryInfo(process: isize, counters: *mut ProcessMemoryCounters, size: u32)
        -> i32;
}

#[cfg(windows)]
pub fn peak_rss_bytes() -> u64 {
    let mut counters: ProcessMemoryCounters = unsafe { std::mem::zeroed() };
    counters.cb = std::mem::size_of::<ProcessMemoryCounters>() as u32;
    let ok = unsafe {
        GetProcessMemoryInfo(
            GetCurrentProcess(),
            &mut counters,
            std::mem::size_of::<ProcessMemoryCounters>() as u32,
        )
    };
    if ok == 0 {
        0
    } else {
        counters.peak_working_set_size as u64
    }
}

#[cfg(not(any(unix, windows)))]
pub fn peak_rss_bytes() -> u64 {
    0
}

#[cfg(test)]
mod tests {
    use super::peak_rss_bytes;

    #[test]
    fn peak_rss_is_available_on_supported_desktop_platforms() {
        #[cfg(any(unix, windows))]
        assert!(peak_rss_bytes() > 0);
    }
}
