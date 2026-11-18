# Security & Resource Limits

## Untrusted Input Limits

All entrypoints enforce strict bounds before allocating buffers:

- `Image::new`: Rejects zero dimensions (`zero_dimension`), images exceeding `MAX_PIXELS = 16_777_216` (`too_large`), and buffers not matching `width * height * 4` (`size_mismatch`).
- `parse_pipeline`: Rejects specifications longer than `1_024` bytes (`too_large`) or containing more than `32` steps (`pipeline_too_long`).
- `ppm::decode_ppm` and `cli::run`: Bound file and slice reads to `67_108_864` bytes (`64 MiB`, `too_large`).
- `FilterError`: Never includes more than 200 characters of user input in formatted error messages.
- `#![forbid(unsafe_code)]` is enforced across the crate.
