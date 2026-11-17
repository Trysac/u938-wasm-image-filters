# trysac-wasm-image-filters

Integer-only RGBA image filters that give bit-identical results natively and in WebAssembly, with Gaussian blur, Sobel edges, sepia, contrast and a filter pipeline.

## Why

Web apps that filter images in the browser get different pixels from their server-side pipeline because floating-point filters round differently across engines and CPU/Wasm targets. `trysac-wasm-image-filters` (`wasm_image_filters`) uses pure integer arithmetic (`i32` accumulators and explicit rounding shifts/divisions) so every target produces the exact same bytes and FNV-1a 64-bit digest.

## Usage

```rust
use wasm_image_filters::{Filter, Image, apply, parse_pipeline, pipeline};

let img = Image::new(1, 1, vec![255, 0, 0, 255]).unwrap();
let gray = apply(&img, &Filter::Grayscale);
assert_eq!(gray.data(), &[54, 54, 54, 255]);

let steps = parse_pipeline("grayscale,blur3,threshold:128").unwrap();
let out = pipeline(&img, &steps);
assert_eq!(out.digest(), 9_088_023_541_897_611_592_u64);
```

## CLI

Apply a filter pipeline to a binary PPM (`P6`, `maxval 255`) image:

```text
wasm-image-filters apply input.ppm --filters grayscale,blur3,sobel --out output.ppm
```
