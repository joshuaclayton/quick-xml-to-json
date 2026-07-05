//! Measure peak heap allocated during conversion, per fixture.
//!
//! Wraps the system allocator to track live and peak heap bytes, then converts a
//! document once per API and reports the peak allocation attributable to conversion:
//!
//! - buffered path: streams from the file through `xml_to_json` into `io::sink()`,
//!   so neither input nor output is held in memory
//! - slice path: input is read into memory *before* the measurement window (it is
//!   the caller's, by definition), output still goes to `io::sink()`
//!
//! Usage: `cargo run --release --example peak_memory -- benches/fixtures/nasa.xml`

use quick_xml_to_json::{xml_to_json, xml_to_json_from_slice};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

struct CountingAllocator;

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let live = CURRENT.fetch_add(layout.size(), Ordering::SeqCst) + layout.size();
            PEAK.fetch_max(live, Ordering::SeqCst);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        CURRENT.fetch_sub(layout.size(), Ordering::SeqCst);
    }
}

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Run `convert` and return the peak heap growth (bytes) above the pre-call baseline.
fn measure<F: FnOnce() -> Result<(), quick_xml_to_json::XmlToJsonError>>(
    convert: F,
) -> Result<usize, quick_xml_to_json::XmlToJsonError> {
    let baseline = CURRENT.load(Ordering::SeqCst);
    PEAK.store(baseline, Ordering::SeqCst);

    convert()?;

    Ok(PEAK.load(Ordering::SeqCst).saturating_sub(baseline))
}

#[expect(
    clippy::cast_precision_loss,
    reason = "display-only conversion of small byte counts"
)]
fn mib(bytes: usize) -> f64 {
    bytes as f64 / (1024.0 * 1024.0)
}

#[expect(
    clippy::print_stdout,
    reason = "example binary exists to print measurements"
)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("usage: peak_memory <xml-file>")?;

    let file = std::fs::File::open(&path)?;
    let streamed = measure(|| xml_to_json(file, std::io::sink()))?;

    let bytes = std::fs::read(&path)?;
    let sliced = measure(|| xml_to_json_from_slice(&bytes, std::io::sink()))?;

    println!("{path}: input size {:.2} MiB", mib(bytes.len()));
    println!(
        "  xml_to_json (streaming):     peak heap {streamed:>9} bytes ({:.2} MiB)",
        mib(streamed)
    );
    println!(
        "  xml_to_json_from_slice:      peak heap {sliced:>9} bytes ({:.2} MiB)",
        mib(sliced)
    );

    Ok(())
}
