//! Command-line entrypoint logic for applying filter pipelines to binary PPM (`P6`) images.

use std::fs::File;
use std::io::{Read, Write};

use crate::ppm::{MAX_PPM_BYTES, decode_ppm, encode_ppm};
use crate::{FilterError, parse_pipeline, pipeline};

const USAGE: &str = "Usage: wasm-image-filters apply <in.ppm> --filters <spec> --out <out.ppm>\n";

fn read_bounded_file(path: &str) -> Result<Vec<u8>, Result<FilterError, std::io::Error>> {
    let file = File::open(path).map_err(Err)?;
    let limit = (MAX_PPM_BYTES as u64) + 1;
    let mut buf = Vec::new();
    file.take(limit).read_to_end(&mut buf).map_err(Err)?;
    if buf.len() > MAX_PPM_BYTES {
        return Err(Ok(FilterError::TooLarge));
    }
    Ok(buf)
}

/// Executes the CLI command.
///
/// Returns:
/// - `0` on success.
/// - `1` on domain validation failures (`too_large`, `zero_dimension`, `size_mismatch`).
/// - `2` on usage, I/O, or parse errors (`unknown_filter`, `invalid_parameter`, `pipeline_too_long`, `invalid_ppm`).
pub fn run(args: &[String], stdout: &mut dyn Write, stderr: &mut dyn Write) -> i32 {
    if args.is_empty() {
        let _ = stderr.write_all(USAGE.as_bytes());
        return 2;
    }
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        let _ = stdout.write_all(USAGE.as_bytes());
        return 0;
    }
    if args[0] != "apply" {
        let _ = writeln!(stderr, "unknown command: {}", args[0]);
        let _ = stderr.write_all(USAGE.as_bytes());
        return 2;
    }

    let mut input_path: Option<&str> = None;
    let mut filters_spec: Option<&str> = None;
    let mut output_path: Option<&str> = None;

    let mut i = 1usize;
    while i < args.len() {
        match args[i].as_str() {
            "--filters" => {
                if i + 1 >= args.len() {
                    let _ = writeln!(stderr, "missing value for --filters");
                    return 2;
                }
                filters_spec = Some(&args[i + 1]);
                i += 2;
            }
            "--out" => {
                if i + 1 >= args.len() {
                    let _ = writeln!(stderr, "missing value for --out");
                    return 2;
                }
                output_path = Some(&args[i + 1]);
                i += 2;
            }
            arg if arg.starts_with('-') => {
                let _ = writeln!(stderr, "unknown flag: {arg}");
                return 2;
            }
            positional => {
                if input_path.is_some() {
                    let _ = writeln!(stderr, "unexpected positional argument: {positional}");
                    return 2;
                }
                input_path = Some(positional);
                i += 1;
            }
        }
    }

    let (Some(in_path), Some(spec), Some(out_path)) = (input_path, filters_spec, output_path)
    else {
        let _ = stderr.write_all(USAGE.as_bytes());
        return 2;
    };

    let filters = match parse_pipeline(spec) {
        Ok(f) => f,
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            return classify_error(&err);
        }
    };

    let raw_bytes = match read_bounded_file(in_path) {
        Ok(b) => b,
        Err(Ok(domain_err)) => {
            let _ = writeln!(stderr, "{domain_err}");
            return classify_error(&domain_err);
        }
        Err(Err(io_err)) => {
            let _ = writeln!(stderr, "io_error: {io_err}");
            return 2;
        }
    };

    let img = match decode_ppm(&raw_bytes) {
        Ok(image) => image,
        Err(err) => {
            let _ = writeln!(stderr, "{err}");
            return classify_error(&err);
        }
    };

    let filtered = pipeline(&img, &filters);
    let encoded = encode_ppm(&filtered);
    if let Err(io_err) = std::fs::write(out_path, encoded) {
        let _ = writeln!(stderr, "io_error: {io_err}");
        return 2;
    }
    let _ = writeln!(stdout, "digest 0x{:016x}", filtered.digest());
    0
}

const fn classify_error(err: &FilterError) -> i32 {
    match err {
        FilterError::SizeMismatch { .. } | FilterError::ZeroDimension | FilterError::TooLarge => 1,
        FilterError::UnknownFilter(_)
        | FilterError::InvalidParameter(_)
        | FilterError::PipelineTooLong(_)
        | FilterError::InvalidPpm(_) => 2,
    }
}
