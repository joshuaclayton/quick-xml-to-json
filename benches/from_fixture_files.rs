use criterion::{BenchmarkId, Criterion, Throughput};
use criterion::{criterion_group, criterion_main};
use quick_xml_to_json::xml_to_json_from_bufread;
use std::{fs, io::Cursor, path::PathBuf};

fn from_fixture_files(c: &mut Criterion) {
    let mut group = c.benchmark_group("Fixture file benchmarks");

    // assets sourced from https://aiweb.cs.washington.edu/research/projects/xmltk/xmldata/www/repository.html
    let files = [
        // ~500kb
        "SigmodRecord.xml",
        // ~1.8mb
        "mondial-3.0.xml",
        // ~5.4mb
        "orders.xml",
        // ~25.1mb
        "nasa.xml",
    ];

    for file in files {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let test_file = format!("benches/fixtures/{file}");

        path.push(test_file);
        let bytes = fs::read(&path).expect("Failed to read input file");

        group.throughput(Throughput::Bytes(bytes.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(file), &bytes, |b, data| {
            let mut out = Vec::with_capacity(data.len() * 2);
            let mut reader = Cursor::new(data.as_slice());
            let mut writer = Cursor::new(&mut out);

            b.iter(|| {
                reader.set_position(0);
                writer.get_mut().clear();
                writer.set_position(0);

                xml_to_json_from_bufread(&mut reader, &mut writer).unwrap();
            });
        });
    }

    group.finish();
}

criterion_group!(benches, from_fixture_files);
criterion_main!(benches);
