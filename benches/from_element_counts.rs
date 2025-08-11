use criterion::{BenchmarkId, Criterion, Throughput};
use criterion::{criterion_group, criterion_main};
use quick_xml_to_json::xml_to_json;
use std::io::Cursor;

fn generate_users_xml(n: usize) -> String {
    let users: String = (1..=n)
        .map(|i| format!("    <user id=\"{i}\">User {i}</user>\n"))
        .collect();

    format!("<users>\n{users}</users>",)
}

fn from_element_counts(c: &mut Criterion) {
    let mut group = c.benchmark_group("xml_to_json");
    for size in [400, 800, 1200, 1600, 2000, 4000, 6000].iter() {
        group.throughput(Throughput::Elements(*size as u64));
        let reader = Cursor::new(generate_users_xml(*size));
        group.bench_with_input(BenchmarkId::from_parameter(size), &reader, |b, s| {
            let mut writer = Cursor::new(Vec::new());

            b.iter(|| xml_to_json(s.clone(), &mut writer));
        });
    }
    group.finish();
}

criterion_group!(benches, from_element_counts,);
criterion_main!(benches);
