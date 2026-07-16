//! Feature-gated counting global allocator (the memory-diet step 0).
//!
//! perf_event/valgrind/heaptrack are unavailable in this environment
//! (perf_event_paranoid=4, ptrace_scope=1) — the repo pattern is
//! feature-gated in-process instrumentation (cf. the `prof` feature).
//! This wraps the System allocator and histograms every allocation by
//! size: EXACT slots for sizes 0..=512 (one-element Vec<FactId> = 4B,
//! stored join keys, small tuples — the diet's suspects all live
//! here), power-of-two classes above. Two views per slot:
//!   - cumulative allocs/bytes (allocator churn)
//!   - live count at the PEAK of global live bytes (RSS attribution)
//! The peak snapshot re-records when live exceeds the previous peak by
//! 4MB (hysteresis keeps it off the hot path). Realloc is accounted as
//! free(old)+alloc(new) but performed by System::realloc, so program
//! behavior matches an uninstrumented run.
//!
//! Build: cargo build --release -p seine-harness --features alloc_stats
//! Dump: "ALLOC ..." lines on stderr when the process exits normally.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

/// Exact slots for 0..=512 bytes, then pow2 classes 1KB..=2^(10+POW2-1),
/// with a final catch-all.
const EXACT: usize = 513;
const POW2: usize = 32;
const NSLOTS: usize = EXACT + POW2;

static ALLOCS: [AtomicU64; NSLOTS] = [const { AtomicU64::new(0) }; NSLOTS];
static FREES: [AtomicU64; NSLOTS] = [const { AtomicU64::new(0) }; NSLOTS];
static BYTES: [AtomicU64; NSLOTS] = [const { AtomicU64::new(0) }; NSLOTS];
static AT_PEAK: [AtomicU64; NSLOTS] = [const { AtomicU64::new(0) }; NSLOTS];
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);
/// Peak snapshot hysteresis: only re-walk the slot arrays when live
/// exceeds the recorded peak by this much.
const PEAK_STEP: u64 = 4 << 20;

#[inline]
fn slot(size: usize) -> usize {
    if size < EXACT {
        size
    } else {
        // 513..1024 -> class 0 (<=1KB), 1025..2048 -> 1, ...
        let cls = (usize::BITS - (size - 1).leading_zeros()) as usize - 10;
        EXACT + cls.min(POW2 - 1)
    }
}

fn on_alloc(size: usize) {
    let s = slot(size);
    ALLOCS[s].fetch_add(1, Relaxed);
    BYTES[s].fetch_add(size as u64, Relaxed);
    let live = LIVE_BYTES.fetch_add(size as u64, Relaxed) + size as u64;
    if live > PEAK_BYTES.load(Relaxed) + PEAK_STEP {
        PEAK_BYTES.store(live, Relaxed);
        for i in 0..NSLOTS {
            let net = ALLOCS[i].load(Relaxed).wrapping_sub(FREES[i].load(Relaxed));
            AT_PEAK[i].store(net, Relaxed);
        }
    }
}

fn on_free(size: usize) {
    FREES[slot(size)].fetch_add(1, Relaxed);
    LIVE_BYTES.fetch_sub(size as u64, Relaxed);
}

pub struct CountingAlloc;

unsafe impl GlobalAlloc for CountingAlloc {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc(layout);
        if !p.is_null() {
            on_alloc(layout.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let p = System.alloc_zeroed(layout);
        if !p.is_null() {
            on_alloc(layout.size());
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        on_free(layout.size());
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = System.realloc(ptr, layout, new_size);
        if !p.is_null() {
            on_free(layout.size());
            on_alloc(new_size);
        }
        p
    }
}

/// Drop guard (the ProfDump pattern): dumps the histogram to stderr
/// when main returns.
pub struct AllocDump;

impl Drop for AllocDump {
    fn drop(&mut self) {
        let mut rows: Vec<(String, u64, u64, u64, u64, u64)> = Vec::new();
        for i in 0..NSLOTS {
            let allocs = ALLOCS[i].load(Relaxed);
            if allocs == 0 {
                continue;
            }
            let bytes = BYTES[i].load(Relaxed);
            let at_peak = AT_PEAK[i].load(Relaxed);
            let (label, peak_bytes) = if i < EXACT {
                (format!("{}", i), at_peak * i as u64)
            } else {
                let hi = 1u64 << (i - EXACT + 10);
                // pow2 class peak bytes: approximate with the class's
                // mean cumulative alloc size.
                let mean = bytes / allocs.max(1);
                (format!("<={}", hi), at_peak * mean)
            };
            rows.push((label, allocs, bytes, at_peak, peak_bytes, i as u64));
        }
        eprintln!("ALLOC size    cum_allocs      cum_bytes   live_at_peak  peak_bytes(est)");
        for (label, allocs, bytes, at_peak, peak_bytes, _) in &rows {
            eprintln!(
                "ALLOC {:>7} {:>12} {:>14} {:>12} {:>14}",
                label, allocs, bytes, at_peak, peak_bytes
            );
        }
        let ta: u64 = rows.iter().map(|r| r.1).sum();
        let tb: u64 = rows.iter().map(|r| r.2).sum();
        let tp: u64 = rows.iter().map(|r| r.4).sum();
        eprintln!(
            "ALLOC TOTAL allocs={} bytes={} peak_live_bytes={} peak_attributed={} final_live={}",
            ta,
            tb,
            PEAK_BYTES.load(Relaxed),
            tp,
            LIVE_BYTES.load(Relaxed)
        );
    }
}
