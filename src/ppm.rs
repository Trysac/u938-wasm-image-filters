//! Binary PPM (`P6`) reader and writer with `maxval == 255`.

use crate::{FilterError, Image};

/// Maximum allowed input size in bytes for a PPM file (`64 MiB`).
pub const MAX_PPM_BYTES: usize = 67_108_864;

const fn is_ppm_whitespace(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn next_header_token(bytes: &[u8], pos: &mut usize) -> Result<(usize, usize), FilterError> {
    let len = bytes.len();
    while *pos < len {
        let b = bytes[*pos];
        if is_ppm_whitespace(b) {
            *pos += 1;
        } else if b == b'#' {
            while *pos < len && bytes[*pos] != b'\n' {
                *pos += 1;
            }
        } else {
            break;
        }
    }

    if *pos >= len {
        return Err(FilterError::InvalidPpm(
            "unexpected end of PPM header".to_string(),
        ));
    }

    let start = *pos;
    while *pos < len && !is_ppm_whitespace(bytes[*pos]) && bytes[*pos] != b'#' {
        *pos += 1;
    }
    Ok((start, *pos))
}

fn parse_u32_ascii(bytes: &[u8], label: &str) -> Result<u32, FilterError> {
    let s = std::str::from_utf8(bytes)
        .map_err(|_| FilterError::InvalidPpm(format!("non-ASCII {label}")))?;
    s.parse::<u32>()
        .map_err(|_| FilterError::InvalidPpm(format!("invalid {label}")))
}

/// Decodes a binary PPM (`P6`) byte slice into an RGBA8 [`Image`] with alpha set to `255`.
///
/// # Errors
///
/// - [`FilterError::TooLarge`] if `bytes.len() > 67_108_864` or dimensions exceed `MAX_PIXELS`.
/// - [`FilterError::ZeroDimension`] if the header specifies zero width or height.
/// - [`FilterError::InvalidPpm`] if the header is not `P6`, `maxval != 255`, or pixel data length mismatches.
pub fn decode_ppm(bytes: &[u8]) -> Result<Image, FilterError> {
    if bytes.len() > MAX_PPM_BYTES {
        return Err(FilterError::TooLarge);
    }
    let mut pos = 0usize;
    let (m_start, m_end) = next_header_token(bytes, &mut pos)?;
    if &bytes[m_start..m_end] != b"P6" {
        return Err(FilterError::InvalidPpm(
            "expected P6 magic header".to_string(),
        ));
    }

    let (w_start, w_end) = next_header_token(bytes, &mut pos)?;
    let width = parse_u32_ascii(&bytes[w_start..w_end], "width")?;

    let (h_start, h_end) = next_header_token(bytes, &mut pos)?;
    let height = parse_u32_ascii(&bytes[h_start..h_end], "height")?;

    let (max_start, max_end) = next_header_token(bytes, &mut pos)?;
    let maxval = parse_u32_ascii(&bytes[max_start..max_end], "maxval")?;
    if maxval != 255 {
        return Err(FilterError::InvalidPpm(
            "only maxval 255 is supported".to_string(),
        ));
    }

    if pos >= bytes.len() || !is_ppm_whitespace(bytes[pos]) {
        return Err(FilterError::InvalidPpm(
            "missing single whitespace after maxval".to_string(),
        ));
    }
    pos += 1;

    if width == 0 || height == 0 {
        return Err(FilterError::ZeroDimension);
    }
    let pixels = u64::from(width) * u64::from(height);
    if pixels > crate::MAX_PIXELS {
        return Err(FilterError::TooLarge);
    }
    let expected_rgb = usize::try_from(pixels * 3).map_err(|_| FilterError::TooLarge)?;
    let body = &bytes[pos..];
    if body.len() != expected_rgb {
        return Err(FilterError::InvalidPpm(format!(
            "expected {expected_rgb} RGB bytes, got {}",
            body.len()
        )));
    }

    let expected_rgba = usize::try_from(pixels * 4).map_err(|_| FilterError::TooLarge)?;
    let mut rgba = Vec::with_capacity(expected_rgba);
    for rgb in body.as_chunks::<3>().0 {
        rgba.push(rgb[0]);
        rgba.push(rgb[1]);
        rgba.push(rgb[2]);
        rgba.push(255);
    }

    Image::new(width, height, rgba)
}

/// Encodes an [`Image`] as a binary PPM (`P6`) byte vector, dropping the alpha channel.
#[must_use]
pub fn encode_ppm(img: &Image) -> Vec<u8> {
    let header = format!("P6\n{} {}\n255\n", img.width(), img.height());
    let rgb_len = (img.data().len() / 4) * 3;
    let mut out = Vec::with_capacity(header.len() + rgb_len);
    out.extend_from_slice(header.as_bytes());
    for px in img.data().as_chunks::<4>().0 {
        out.push(px[0]);
        out.push(px[1]);
        out.push(px[2]);
    }
    out
}
