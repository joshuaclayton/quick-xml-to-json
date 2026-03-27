use quick_xml_to_json::xml_to_json;
use std::fmt::Write as _;
use std::time::Instant;

fn generate_xml(n: usize) -> String {
    let mut xml = String::from("<root>\n");
    for i in 1..=n {
        writeln!(
            xml,
            "    <item id=\"{i}\" category=\"test\">Item number {i}</item>"
        )
        .unwrap_or_default();
    }
    xml.push_str("</root>");
    xml
}

/// Catch catastrophic performance regressions (algorithmic changes, lost optimizations).
///
/// These thresholds are intentionally generous (~20x headroom over typical fast hardware)
/// so they remain stable across CI environments and slow machines. They are NOT meant to
/// detect small constant-factor regressions — use `cargo bench` for that.
#[test]
fn throughput_does_not_regress_catastrophically() {
    let xml = generate_xml(10_000);
    let input = xml.as_bytes();

    // Warm up the allocator
    let mut out = Vec::with_capacity(input.len() * 2);
    xml_to_json(input, &mut out).unwrap_or_default();
    out.clear();

    // Timed run
    let start = Instant::now();
    xml_to_json(input, &mut out).unwrap_or_default();
    let elapsed = start.elapsed();

    // 10k elements with attributes and text currently runs in ~1-2ms in release on fast
    // hardware, but debug builds on CI runners can be 50-100x slower. Threshold of 500ms
    // still catches algorithmic regressions (e.g. O(n^2)) while staying stable in debug
    // mode on slow CI machines.
    assert!(
        elapsed.as_millis() < 500,
        "10k element conversion took {elapsed:?}, expected < 500ms — possible performance regression"
    );
}

#[test]
fn deeply_nested_xml_does_not_regress() {
    // Build deeply nested XML to stress the stack path
    let depth = 200;
    let mut xml = String::new();
    for i in 0..depth {
        write!(xml, "<level{i}>").unwrap_or_default();
    }
    xml.push_str("leaf");
    for i in (0..depth).rev() {
        write!(xml, "</level{i}>").unwrap_or_default();
    }

    let input = xml.as_bytes();
    let mut out = Vec::with_capacity(input.len() * 3);

    let start = Instant::now();
    xml_to_json(input, &mut out).unwrap_or_default();
    let elapsed = start.elapsed();

    assert!(
        elapsed.as_millis() < 100,
        "deeply nested conversion took {elapsed:?}, expected < 100ms — possible performance regression"
    );
}
