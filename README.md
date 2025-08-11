# quick-xml-to-json

[![Crates.io](https://img.shields.io/crates/v/quick-xml-to-json)](https://crates.io/crates/quick-xml-to-json)
[![Documentation](https://docs.rs/quick-xml-to-json/badge.svg)](https://docs.rs/quick-xml-to-json)

High-performance XML to JSON converter built on top of [quick-xml](https://github.com/tafia/quick-xml).

This crate provides a fast, memory-efficient way to convert XML documents to JSON format, leveraging quick-xml's high-performance XML parsing capabilities.

## Features

- **High Performance**: Built on quick-xml for maximum parsing speed
- **Memory Efficient**: Minimal memory allocations with configurable buffer sizes
- **Streaming**: Processes XML as a stream without loading entire documents into memory
- **Attribute Support**: Preserves XML attributes in JSON output
- **Text Node Handling**: Properly handles text content within elements
- **Error Handling**: Comprehensive error types with `thiserror`

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
quick-xml-to-json = "0.1.0"
```

## Usage

### Basic Conversion

```rust
use quick_xml_to_json::xml_to_json;

let xml = r#"<users count="3">
  <user age="40">Jane Doe</user>
  <user age="42">John Doe</user>
</users>"#;

let mut output = Vec::new();
xml_to_json(xml.as_bytes(), &mut output)?;

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

## Performance

This crate is designed for high-performance conversion of XML to JSON.

### Benchmark Results

XML fixture files have been sourced from <https://aiweb.cs.washington.edu/research/projects/xmltk/xmldata/www/repository.html>.

On a 12-core MacBook Pro M4, throughput benchmarks vary but sit between 215 MiB/s and 340 MiB/s with minimal RAM usage (~3 MiB).

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
- **serde**: JSON serialization support
- **serde_json**: JSON formatting
- **thiserror**: Custom error types

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a pull request.
