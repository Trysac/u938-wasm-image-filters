# Architecture

## Overview

`trysac-wasm-image-filters` (library `wasm_image_filters`) provides deterministic RGBA8 image processing in pure Rust using only the standard library.

## Invariant: Integer-Only Filter Arithmetic

No floating-point types (`f32`, `f64`) are used anywhere in the crate. Every filter formula uses fixed-point integer arithmetic over `i32` accumulators:

- **Grayscale**: BT.709 weights scaled to 256: `(54 * R + 183 * G + 19 * B + 128) >> 8`.
- **Invert**: `255 - c`.
- **Brightness**: `clamp(v + d, 0, 255)` for `d` in `-255..=255`.
- **Contrast**: `((v - 128) * c + 500 * sign) / 1000 + 128` rounded half away from zero, clamped to `0..=255`.
- **Threshold**: Grayscale luminance `>= t` maps to `255`, otherwise `0`.
- **Sepia**: Integer matrix `r = (393 R + 769 G + 189 B) / 1000`, `g = (349 R + 686 G + 168 B) / 1000`, `b = (272 R + 534 G + 131 B) / 1000`, truncated toward zero and clamped to `0..=255`.
- **GaussianBlur3**: 3x3 outer product of `[1, 2, 1]` with `(sum + 8) >> 4` and clamped edge coordinates.
- **GaussianBlur5**: 5x5 outer product of `[1, 4, 6, 4, 1]` with `(sum + 128) >> 8` and clamped edge coordinates.
- **BoxBlur**: `(2r + 1)^2` window (`r` in `1..=16`) with `(sum + n / 2) / n` and clamped edge coordinates.
- **Sobel**: 3x3 horizontal and vertical kernels over BT.709 grayscale luminance with `min(255, |gx| + |gy|)`.

Alpha bytes are always preserved untouched across all filters.
