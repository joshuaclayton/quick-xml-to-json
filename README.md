# quick-xml-to-json

[![Crates.io](https://img.shields.io/crates/v/quick-xml-to-json)](https://crates.io/crates/quick-xml-to-json)
[![Documentation](https://docs.rs/quick-xml-to-json/badge.svg)](https://docs.rs/quick-xml-to-json)

High-performance XML to JSON converter built on top of [quick-xml](https://github.com/tafia/quick-xml).

This crate provides a fast, memory-efficient way to convert XML documents to JSON format, leveraging quick-xml's high-performance XML parsing capabilities.

## Features

- **High Performance**: Built on quick-xml for maximum parsing speed
- **Zero-Copy Option**: `xml_to_json_from_slice` borrows events directly from in-memory input
- **Streaming**: Processes XML as a stream without building a DOM, keeping memory usage flat
- **Attribute Support**: Preserves XML attributes in JSON output
- **Entity Handling**: Resolves predefined entities and numeric character references
- **Error Handling**: Comprehensive error types with `thiserror`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
quick-xml-to-json = "0.3.0"
```

## Usage

Three entry points, chosen by input shape:

- `xml_to_json_from_slice(&[u8], impl Write)` — fastest; use when the document is already in memory
- `xml_to_json_from_bufread(impl BufRead, impl Write)` — use when the input is already buffered
- `xml_to_json(impl Read, impl Write)` — wraps any reader in a `BufReader` for you

### Basic Conversion

```rust
use quick_xml_to_json::xml_to_json_from_slice;

let xml = r#"<users count="3">
  <user age="40">Jane Doe</user>
  <user age="42">John Doe</user>
</users>"#;

let mut output = Vec::new();
xml_to_json_from_slice(xml.as_bytes(), &mut output)?;

// output now contains the JSON bytes
let json_string = String::from_utf8(output)?;
println!("{}", json_string);
```

### JSON Output Format

The crate converts XML to JSON using a specific format that preserves structure:

- **Attributes** are prefixed with `@` (e.g., `@id="value"`)
- **Text content** is stored under the `#t` key
- **Child elements** are stored under the `#c` key as an array

Example XML:

```xml
<root id="main">
  <child>Hello World</child>
  <empty attr="value" />
</root>
```

Becomes:

```json
{
  "root": {
    "@id": "main",
    "#c": [
      {
        "child": {
          "#t": "Hello World"
        }
      },
      {
        "empty": {
          "@attr": "value"
        }
      }
    ]
  }
}
```

### Text, Entities, and Whitespace Semantics

- An element's character data is emitted as a single `#t` value, concatenated in
  document order. For mixed content (`<a>pre <b/>post</a>`), text before and after
  child elements is joined into one `#t` alongside `#c`.
- Leading and trailing whitespace of an element's text is trimmed; interior
  whitespace (including newlines) is preserved exactly.
- Predefined entities (`&amp;`, `&lt;`, `&gt;`, `&quot;`, `&apos;`) and numeric
  character references (`&#65;`, `&#x41;`) are resolved in both text and attribute
  values. Named entities that cannot be resolved (e.g. DTD-defined entities like
  `&uuml;`) pass through verbatim rather than erroring or being dropped. Malformed
  character references (e.g. `&#xZZ;`) are errors.
- CDATA sections are ignored.

## Performance

XML fixture files have been sourced from <https://aiweb.cs.washington.edu/research/projects/xmltk/xmldata/www/repository.html>.

Measured with criterion on a 12-core MacBook Pro M4:

| Fixture | Size | `xml_to_json_from_bufread` | `xml_to_json_from_slice` |
|---|---|---|---|
| SigmodRecord.xml | ~500 KB | 354 MiB/s | 405 MiB/s |
| mondial-3.0.xml | ~1.8 MB | 416 MiB/s | 429 MiB/s |
| orders.xml | ~5.4 MB | 393 MiB/s | 466 MiB/s |
| nasa.xml | ~25.1 MB | 414 MiB/s | 467 MiB/s |

On synthetic documents of many small elements, the buffered path sustains
9.4–9.8 million elements per second.

Run benchmarks with:

```sh
cargo bench
```

## Test Suite

Run the test suite with:

```sh
cargo test
```

## Dependencies

- **quick-xml**: High-performance XML parsing
- **simdutf8**: SIMD-accelerated UTF-8 validation
- **thiserror**: Custom error types

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.
