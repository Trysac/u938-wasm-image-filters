# wasm-image-filters

[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/Trysac/u938-wasm-image-filters/actions/workflows/ci.yml/badge.svg)](https://github.com/Trysac/u938-wasm-image-filters/actions/workflows/ci.yml)

Integer-only RGBA image filters that give bit-identical results natively and in WebAssembly, with Gaussian blur, Sobel edges, sepia, contrast and a filter pipeline

## Problem

Describe in two or three sentences the concrete problem this utility solves, who hits it, and what
they do today without it. Replace this paragraph when the first real feature lands.

## Install

```sh
cargo add trysac-wasm-image-filters --git https://github.com/Trysac/u938-wasm-image-filters
```

Requires Rust 1.88 or newer. No dependencies outside the standard library.

## Usage

```rust
use wasm_image_filters::{Error, normalize};

// normalize trims surrounding whitespace and lower-cases the text.
assert_eq!(normalize("  Hello World ").as_deref(), Ok("hello world"));

// Blank input is rejected explicitly instead of returning an empty string.
assert_eq!(wasm_image_filters::normalize("   "), Err(Error::Blank));
```

## Documentation

- [Architecture](docs/ARCHITECTURE.md) and [decisions](docs/decisions/)
- [Testing](docs/TESTING.md) and [security](docs/SECURITY.md)
- [Project record](docs/PROJECT.md) and [stack](docs/STACK.md)
- [Changelog](CHANGELOG.md) and [contributing guide](CONTRIBUTING.md)

## License

[MIT](LICENSE) (c) 2026 Miguel Gomez
