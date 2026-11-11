//! Diagnostic measurement support. Preserve the original allocator and include
//! every Rust worker thread; authoritative latency binaries remain unchanged.
use std::alloc::{GlobalAlloc, Layout};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering::Relaxed};

static ACTIVE: AtomicBool = AtomicBool::new(false);
static REQUESTS: AtomicU64 = AtomicU64::new(0);
static REQUESTED: AtomicU64 = AtomicU64::new(0);
static FREED: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicU64 = AtomicU64::new(0);
static PEAK: AtomicU64 = AtomicU64::new(0);
static UNDERFLOWS: AtomicU64 = AtomicU64::new(0);

pub struct Tracked<A>(pub A);

fn request(bytes: usize) {
    if ACTIVE.load(Relaxed) {
        REQUESTS.fetch_add(1, Relaxed);
        REQUESTED.fetch_add(bytes as u64, Relaxed);
    }
}
fn acquired(bytes: usize) {
    let live = LIVE.fetch_add(bytes as u64, Relaxed) + bytes as u64;
    if ACTIVE.load(Relaxed) {
        PEAK.fetch_max(live, Relaxed);
    }
}
fn released(bytes: usize) {
    // Account for requested Rust heap bytes, not allocator internals or mmap.
    if LIVE
        .fetch_update(Relaxed, Relaxed, |n| n.checked_sub(bytes as u64))
        .is_err()
    {
        UNDERFLOWS.fetch_add(1, Relaxed);
    }
    if ACTIVE.load(Relaxed) {
        FREED.fetch_add(bytes as u64, Relaxed);
    }
}

// SAFETY: forward every pointer, alignment, size and result unchanged to the
// original allocator. Observation uses allocation-free atomic operations.
unsafe impl<A: GlobalAlloc> GlobalAlloc for Tracked<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        request(layout.size());
        let result = unsafe { self.0.alloc(layout) };
        if !result.is_null() {
            acquired(layout.size());
        }
        result
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        request(layout.size());
        let result = unsafe { self.0.alloc_zeroed(layout) };
        if !result.is_null() {
            acquired(layout.size());
        }
        result
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        request(size);
        let result = unsafe { self.0.realloc(pointer, layout, size) };
        if !result.is_null() {
            released(layout.size());
            acquired(size);
        }
        result
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { self.0.dealloc(pointer, layout) };
        released(layout.size());
    }
}

#[cfg(target_os = "macos")]
mod os {
    use serde::Serialize;

    // Exact RUSAGE_INFO_V4 layout from the installed macOS SDK. The companion
    // C probe verifies sizeof=296 and offsetof(ri_instructions)=248.
    #[repr(C)]
    #[derive(Clone, Copy, Default)]
    pub struct Snapshot {
        uuid: [u8; 16],
        user: u64,
        system: u64,
        idle_wakeups: u64,
        interrupt_wakeups: u64,
        pageins: u64,
        wired: u64,
        resident: u64,
        footprint: u64,
        start: u64,
        exit: u64,
        child_user: u64,
        child_system: u64,
        child_idle: u64,
        child_interrupt: u64,
        child_pageins: u64,
        child_elapsed: u64,
        disk_read: u64,
        disk_written: u64,
        qos_default: u64,
        qos_maintenance: u64,
        qos_background: u64,
        qos_utility: u64,
        qos_legacy: u64,
        qos_initiated: u64,
        qos_interactive: u64,
        billed_system: u64,
        serviced_system: u64,
        logical_writes: u64,
        lifetime_peak_footprint: u64,
        instructions: u64,
        cycles: u64,
        billed_energy: u64,
        serviced_energy: u64,
        interval_peak_footprint: u64,
        runnable: u64,
    }
    const _: () = assert!(std::mem::size_of::<Snapshot>() == 296);
    const _: () = assert!(std::mem::offset_of!(Snapshot, instructions) == 248);
    #[repr(C)]
    struct Timebase {
        numer: u32,
        denom: u32,
    }
    #[link(name = "proc")]
    unsafe extern "C" {
        fn proc_pid_rusage(pid: i32, flavor: i32, buffer: *mut std::ffi::c_void) -> i32;
        fn getpid() -> i32;
        fn mach_timebase_info(info: *mut Timebase) -> i32;
    }
    impl Snapshot {
        pub fn read() -> Option<Self> {
            let mut value = Self::default();
            // SAFETY: caller-owned, correctly sized/aligned SDK structure.
            let result = unsafe { proc_pid_rusage(getpid(), 4, (&mut value as *mut Self).cast()) };
            (result == 0).then_some(value)
        }
        pub fn difference(self, end: Self) -> Option<Hardware> {
            let mut timebase = Timebase { numer: 0, denom: 0 };
            // ri_user_time and ri_system_time use Mach ticks, verified against
            // getrusage timeval by the SDK C probe; convert rather than assume ns.
            if unsafe { mach_timebase_info(&mut timebase) } != 0 || timebase.denom == 0 {
                return None;
            }
            let seconds = f64::from(timebase.numer) / f64::from(timebase.denom) * 1e-9;
            let instructions = end.instructions.checked_sub(self.instructions)?;
            let cycles = end.cycles.checked_sub(self.cycles)?;
            Some(Hardware {
                instructions_retired: instructions,
                cycles,
                instructions_per_cycle: (cycles != 0)
                    .then_some(instructions as f64 / cycles as f64),
                user_cpu_seconds: end.user.checked_sub(self.user)? as f64 * seconds,
                system_cpu_seconds: end.system.checked_sub(self.system)? as f64 * seconds,
                resident_bytes_before: self.resident,
                resident_bytes_after: end.resident,
                physical_footprint_bytes_before: self.footprint,
                physical_footprint_bytes_after: end.footprint,
                lifetime_peak_physical_footprint_bytes_after: end.lifetime_peak_footprint,
                pageins: end.pageins.checked_sub(self.pageins)?,
            })
        }
    }
    #[derive(Serialize)]
    pub struct Hardware {
        instructions_retired: u64,
        cycles: u64,
        instructions_per_cycle: Option<f64>,
        user_cpu_seconds: f64,
        system_cpu_seconds: f64,
        resident_bytes_before: u64,
        resident_bytes_after: u64,
        physical_footprint_bytes_before: u64,
        physical_footprint_bytes_after: u64,
        lifetime_peak_physical_footprint_bytes_after: u64,
        pageins: u64,
    }
}

#[cfg(not(target_os = "macos"))]
mod os {
    pub struct Snapshot;
    impl Snapshot {
        pub fn read() -> Option<Self> {
            None
        }
        pub fn difference(self, _: Self) -> Option<()> {
            None
        }
    }
    pub type Hardware = ();
}

pub struct Window {
    before: Option<os::Snapshot>,
    heap_before: u64,
    underflows: u64,
}
impl Window {
    pub fn begin() -> Self {
        assert!(!ACTIVE.load(Relaxed), "measurement windows cannot overlap");
        let before = os::Snapshot::read();
        let heap_before = LIVE.load(Relaxed);
        REQUESTS.store(0, Relaxed);
        REQUESTED.store(0, Relaxed);
        FREED.store(0, Relaxed);
        PEAK.store(heap_before, Relaxed);
        let underflows = UNDERFLOWS.load(Relaxed);
        ACTIVE.store(true, std::sync::atomic::Ordering::Release);
        Self {
            before,
            heap_before,
            underflows,
        }
    }
    pub fn finish(self) -> Measurement {
        let end = os::Snapshot::read();
        ACTIVE.store(false, std::sync::atomic::Ordering::Release);
        let peak = PEAK.load(Relaxed);
        Measurement {
            hardware: self.before.zip(end).and_then(|(a, b)| a.difference(b)),
            allocation_requests: REQUESTS.load(Relaxed),
            requested_allocation_bytes: REQUESTED.load(Relaxed),
            freed_allocation_bytes: FREED.load(Relaxed),
            live_requested_heap_bytes_before: self.heap_before,
            live_requested_heap_bytes_after: LIVE.load(Relaxed),
            peak_live_requested_heap_bytes: peak,
            peak_additional_live_requested_heap_bytes: peak.saturating_sub(self.heap_before),
            allocator_accounting_underflows: UNDERFLOWS.load(Relaxed) - self.underflows,
            allocator_lifetime_accounting_underflows: UNDERFLOWS.load(Relaxed),
        }
    }
}

#[derive(serde::Serialize)]
pub struct Measurement {
    hardware: Option<os::Hardware>,
    allocation_requests: u64,
    requested_allocation_bytes: u64,
    freed_allocation_bytes: u64,
    live_requested_heap_bytes_before: u64,
    live_requested_heap_bytes_after: u64,
    peak_live_requested_heap_bytes: u64,
    peak_additional_live_requested_heap_bytes: u64,
    allocator_accounting_underflows: u64,
    allocator_lifetime_accounting_underflows: u64,
}
