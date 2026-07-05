use criterion::{BenchmarkId, Criterion, Throughput};
use criterion::{criterion_group, criterion_main};
use quick_xml_to_json::{xml_to_json_from_bufread, xml_to_json_from_slice};
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
        let xml = generate_users_xml(*size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &xml, |b, s| {
            let mut reader = Cursor::new(s.as_bytes());
            let mut out = Vec::new();
            let mut writer = Cursor::new(&mut out);

            b.iter(|| {
                reader.set_position(0);
                writer.get_mut().clear();
                writer.set_position(0);

                xml_to_json_from_bufread(&mut reader, &mut writer).unwrap();
            });
        });

        group.bench_with_input(BenchmarkId::new("slice", size), &xml, |b, s| {
            let mut out = Vec::new();

            b.iter(|| {
                out.clear();
                xml_to_json_from_slice(s.as_bytes(), &mut out).unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, from_element_counts,);
criterion_main!(benches);
