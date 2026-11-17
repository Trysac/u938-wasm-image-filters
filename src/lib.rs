// The README is the crate documentation, so its usage example runs as a doc test.
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use std::fmt;

pub mod cli;
pub mod ppm;

/// Maximum allowed pixels in an [`Image`] (`4096 * 4096 = 16_777_216`).
pub const MAX_PIXELS: u64 = 16_777_216;

/// Maximum byte length of a pipeline specification string.
pub const MAX_PIPELINE_SPEC_BYTES: usize = 1_024;

/// Maximum number of filter steps in a parsed pipeline.
pub const MAX_PIPELINE_STEPS: usize = 32;

/// Errors returned by image validation, pipeline parsing, and PPM decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FilterError {
    /// The buffer length does not equal `width * height * 4`.
    SizeMismatch {
        /// Expected byte length.
        expected: usize,
        /// Actual byte length.
        actual: usize,
    },
    /// Either `width` or `height` is zero.
    ZeroDimension,
    /// The image or input exceeds the configured size limit.
    TooLarge,
    /// A filter name in a pipeline specification is unknown.
    UnknownFilter(String),
    /// A filter parameter is missing, malformed, or out of range.
    InvalidParameter(String),
    /// A pipeline specification has more than [`MAX_PIPELINE_STEPS`] steps.
    PipelineTooLong(usize),
    /// A binary PPM (P6) image header or body is invalid.
    InvalidPpm(String),
}

impl FilterError {
    /// Stable machine-readable error code.
    #[must_use]
    pub const fn code(&self) -> &'static str {
        match self {
            Self::SizeMismatch { .. } => "size_mismatch",
            Self::ZeroDimension => "zero_dimension",
            Self::TooLarge => "too_large",
            Self::UnknownFilter(_) => "unknown_filter",
            Self::InvalidParameter(_) => "invalid_parameter",
            Self::PipelineTooLong(_) => "pipeline_too_long",
            Self::InvalidPpm(_) => "invalid_ppm",
        }
    }
}

fn truncate_msg(s: &str) -> &str {
    const LIMIT: usize = 120;
    if s.len() <= LIMIT {
        return s;
    }
    let mut end = LIMIT;
    while !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

impl fmt::Display for FilterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeMismatch { expected, actual } => {
                write!(
                    f,
                    "size_mismatch: expected {expected} bytes, got {actual} bytes"
                )
            }
            Self::ZeroDimension => f.write_str("zero_dimension: width and height must be non-zero"),
            Self::TooLarge => f.write_str("too_large: input exceeds maximum allowed size"),
            Self::UnknownFilter(name) => {
                write!(f, "unknown_filter: {}", truncate_msg(name))
            }
            Self::InvalidParameter(detail) => {
                write!(f, "invalid_parameter: {}", truncate_msg(detail))
            }
            Self::PipelineTooLong(count) => {
                write!(
                    f,
                    "pipeline_too_long: {count} steps exceeds limit of {MAX_PIPELINE_STEPS}"
                )
            }
            Self::InvalidPpm(detail) => {
                write!(f, "invalid_ppm: {}", truncate_msg(detail))
            }
        }
    }
}

impl std::error::Error for FilterError {}

/// An RGBA8 image with straight alpha in row-major order (matching HTML Canvas `ImageData`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    data: Vec<u8>,
}

impl Image {
    /// Validates dimensions and buffer size and creates a new [`Image`].
    ///
    /// # Errors
    ///
    /// - [`FilterError::ZeroDimension`] if `width == 0` or `height == 0`.
    /// - [`FilterError::TooLarge`] if `width * height > 16_777_216`.
    /// - [`FilterError::SizeMismatch`] if `data.len() != width * height * 4`.
    pub fn new(width: u32, height: u32, data: Vec<u8>) -> Result<Self, FilterError> {
        if width == 0 || height == 0 {
            return Err(FilterError::ZeroDimension);
        }
        let pixels = u64::from(width) * u64::from(height);
        if pixels > MAX_PIXELS {
            return Err(FilterError::TooLarge);
        }
        let expected = usize::try_from(pixels * 4).map_err(|_| FilterError::TooLarge)?;
        if data.len() != expected {
            return Err(FilterError::SizeMismatch {
                expected,
                actual: data.len(),
            });
        }
        Ok(Self {
            width,
            height,
            data,
        })
    }

    /// Image width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Image height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// Raw RGBA8 pixel bytes in row-major order.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Computes the deterministic 64-bit FNV-1a digest over `width` (little-endian), `height`
    /// (little-endian), and `data`.
    #[must_use]
    pub fn digest(&self) -> u64 {
        const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
        const PRIME: u64 = 0x0100_0000_01b3;

        let mut hash = OFFSET_BASIS;
        for b in self
            .width
            .to_le_bytes()
            .into_iter()
            .chain(self.height.to_le_bytes())
            .chain(self.data.iter().copied())
        {
            hash ^= u64::from(b);
            hash = hash.wrapping_mul(PRIME);
        }
        hash
    }
}

/// Deterministic integer-only image filter operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    /// BT.709 integer grayscale: `(54 * R + 183 * G + 19 * B + 128) >> 8`.
    Grayscale,
    /// Channel inversion: `255 - c`.
    Invert,
    /// Additive brightness offset in `-255..=255`, clamped to `0..=255`.
    Brightness(i16),
    /// Per-mille contrast factor in `0..=4000` (`1000` is unchanged).
    Contrast(u16),
    /// Luminance threshold at `t` (`luma >= t` becomes `255`, else `0`).
    Threshold(u8),
    /// Integer sepia matrix truncated and clamped to `0..=255`.
    Sepia,
    /// 3x3 Gaussian blur with kernel `[1, 2, 1]` outer product and `(sum + 8) >> 4`.
    GaussianBlur3,
    /// 5x5 Gaussian blur with kernel `[1, 4, 6, 4, 1]` outer product and `(sum + 128) >> 8`.
    GaussianBlur5,
    /// Square box blur of radius `1..=16` (`(2r + 1)^2` window) with `(sum + n / 2) / n`.
    BoxBlur(u8),
    /// 3x3 Sobel edge detector over grayscale luminance with `min(255, |gx| + |gy|)`.
    Sobel,
}

#[inline]
fn clamp_u8(v: i32) -> u8 {
    if v <= 0 {
        0
    } else if v >= 255 {
        255
    } else {
        // v is guaranteed to be within 0..=255.
        u8::try_from(v).unwrap_or(0)
    }
}

#[inline]
const fn luma_bt709(r: u8, g: u8, b: u8) -> u8 {
    let sum = 54 * (r as i32) + 183 * (g as i32) + 19 * (b as i32) + 128;
    let y = sum >> 8;
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    {
        y as u8
    }
}

#[inline]
fn apply_contrast_channel(v: u8, c: u16) -> u8 {
    let c_clamped = i32::from(c.min(4000));
    let prod = (i32::from(v) - 128) * c_clamped;
    let bias = match prod.cmp(&0) {
        std::cmp::Ordering::Greater => 500,
        std::cmp::Ordering::Less => -500,
        std::cmp::Ordering::Equal => 0,
    };
    let adjusted = (prod + bias) / 1000 + 128;
    clamp_u8(adjusted)
}

#[inline]
fn clamp_coord(coord: i32, limit: i32) -> usize {
    let clamped = if coord < 0 {
        0
    } else if coord >= limit {
        limit - 1
    } else {
        coord
    };
    usize::try_from(clamped).unwrap_or(0)
}

fn convolve_rgba<const N: usize>(
    img: &Image,
    radius: i32,
    weights_1d: &[i32; N],
    round_fn: fn(i32) -> u8,
) -> Image {
    let w_usize = usize::try_from(img.width).unwrap_or(0);
    let h_usize = usize::try_from(img.height).unwrap_or(0);
    let w_i32 = i32::try_from(img.width).unwrap_or(0);
    let h_i32 = i32::try_from(img.height).unwrap_or(0);
    let src = &img.data;
    let mut out = vec![0u8; src.len()];

    for y in 0..h_usize {
        let y_i32 = i32::try_from(y).unwrap_or(0);
        for x in 0..w_usize {
            let x_i32 = i32::try_from(x).unwrap_or(0);
            let mut r_sum: i32 = 0;
            let mut g_sum: i32 = 0;
            let mut b_sum: i32 = 0;

            for (ky_idx, &wy) in weights_1d.iter().enumerate() {
                let dy = i32::try_from(ky_idx).unwrap_or(0) - radius;
                let sy = clamp_coord(y_i32 + dy, h_i32);
                let row_offset = sy * w_usize;
                for (kx_idx, &wx) in weights_1d.iter().enumerate() {
                    let dx = i32::try_from(kx_idx).unwrap_or(0) - radius;
                    let sx = clamp_coord(x_i32 + dx, w_i32);
                    let idx = (row_offset + sx) * 4;
                    let weight = wx * wy;
                    r_sum += i32::from(src[idx]) * weight;
                    g_sum += i32::from(src[idx + 1]) * weight;
                    b_sum += i32::from(src[idx + 2]) * weight;
                }
            }

            let dst_idx = (y * w_usize + x) * 4;
            out[dst_idx] = round_fn(r_sum);
            out[dst_idx + 1] = round_fn(g_sum);
            out[dst_idx + 2] = round_fn(b_sum);
            out[dst_idx + 3] = src[dst_idx + 3];
        }
    }

    Image {
        width: img.width,
        height: img.height,
        data: out,
    }
}

fn apply_box_blur(img: &Image, radius: u8) -> Image {
    let r = i32::from(radius.clamp(1, 16));
    let side = 2 * r + 1;
    let n = side * side;
    let half = n / 2;

    let w_usize = usize::try_from(img.width).unwrap_or(0);
    let h_usize = usize::try_from(img.height).unwrap_or(0);
    let w_i32 = i32::try_from(img.width).unwrap_or(0);
    let h_i32 = i32::try_from(img.height).unwrap_or(0);
    let src = &img.data;
    let mut out = vec![0u8; src.len()];

    for y in 0..h_usize {
        let y_i32 = i32::try_from(y).unwrap_or(0);
        for x in 0..w_usize {
            let x_i32 = i32::try_from(x).unwrap_or(0);
            let mut r_sum: i32 = 0;
            let mut g_sum: i32 = 0;
            let mut b_sum: i32 = 0;

            for dy in -r..=r {
                let sy = clamp_coord(y_i32 + dy, h_i32);
                let row_offset = sy * w_usize;
                for dx in -r..=r {
                    let sx = clamp_coord(x_i32 + dx, w_i32);
                    let idx = (row_offset + sx) * 4;
                    r_sum += i32::from(src[idx]);
                    g_sum += i32::from(src[idx + 1]);
                    b_sum += i32::from(src[idx + 2]);
                }
            }

            let dst_idx = (y * w_usize + x) * 4;
            out[dst_idx] = clamp_u8((r_sum + half) / n);
            out[dst_idx + 1] = clamp_u8((g_sum + half) / n);
            out[dst_idx + 2] = clamp_u8((b_sum + half) / n);
            out[dst_idx + 3] = src[dst_idx + 3];
        }
    }

    Image {
        width: img.width,
        height: img.height,
        data: out,
    }
}

fn apply_sobel(img: &Image) -> Image {
    const GX: [[i32; 3]; 3] = [[-1, 0, 1], [-2, 0, 2], [-1, 0, 1]];
    const GY: [[i32; 3]; 3] = [[-1, -2, -1], [0, 0, 0], [1, 2, 1]];

    let w_usize = usize::try_from(img.width).unwrap_or(0);
    let h_usize = usize::try_from(img.height).unwrap_or(0);
    let w_i32 = i32::try_from(img.width).unwrap_or(0);
    let h_i32 = i32::try_from(img.height).unwrap_or(0);
    let src = &img.data;

    let mut luma = vec![0i32; w_usize * h_usize];
    for (i, px) in src.as_chunks::<4>().0.iter().enumerate() {
        luma[i] = i32::from(luma_bt709(px[0], px[1], px[2]));
    }

    let mut out = vec![0u8; src.len()];
    for y in 0..h_usize {
        let y_i32 = i32::try_from(y).unwrap_or(0);
        for x in 0..w_usize {
            let x_i32 = i32::try_from(x).unwrap_or(0);
            let mut gx: i32 = 0;
            let mut gy: i32 = 0;

            for (ky, (gx_row, gy_row)) in GX.iter().zip(GY.iter()).enumerate() {
                let dy = i32::try_from(ky).unwrap_or(0) - 1;
                let sy = clamp_coord(y_i32 + dy, h_i32);
                let row_offset = sy * w_usize;
                for (kx, (&wx, &wy)) in gx_row.iter().zip(gy_row.iter()).enumerate() {
                    let dx = i32::try_from(kx).unwrap_or(0) - 1;
                    let sx = clamp_coord(x_i32 + dx, w_i32);
                    let lum = luma[row_offset + sx];
                    gx += lum * wx;
                    gy += lum * wy;
                }
            }

            let mag = clamp_u8(gx.abs() + gy.abs());
            let dst_idx = (y * w_usize + x) * 4;
            out[dst_idx] = mag;
            out[dst_idx + 1] = mag;
            out[dst_idx + 2] = mag;
            out[dst_idx + 3] = src[dst_idx + 3];
        }
    }

    Image {
        width: img.width,
        height: img.height,
        data: out,
    }
}

/// Applies a single [`Filter`] to an [`Image`], returning a new [`Image`] with alpha preserved.
#[must_use]
pub fn apply(img: &Image, filter: &Filter) -> Image {
    match *filter {
        Filter::Grayscale => {
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                let y = luma_bt709(px[0], px[1], px[2]);
                px[0] = y;
                px[1] = y;
                px[2] = y;
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::Invert => {
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                px[0] = 255 - px[0];
                px[1] = 255 - px[1];
                px[2] = 255 - px[2];
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::Brightness(delta) => {
            let d = i32::from(delta.clamp(-255, 255));
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                px[0] = clamp_u8(i32::from(px[0]) + d);
                px[1] = clamp_u8(i32::from(px[1]) + d);
                px[2] = clamp_u8(i32::from(px[2]) + d);
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::Contrast(c) => {
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                px[0] = apply_contrast_channel(px[0], c);
                px[1] = apply_contrast_channel(px[1], c);
                px[2] = apply_contrast_channel(px[2], c);
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::Threshold(t) => {
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                let y = luma_bt709(px[0], px[1], px[2]);
                let v = if y >= t { 255 } else { 0 };
                px[0] = v;
                px[1] = v;
                px[2] = v;
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::Sepia => {
            let mut out = img.data.clone();
            for px in out.as_chunks_mut::<4>().0 {
                let r = i32::from(px[0]);
                let g = i32::from(px[1]);
                let b = i32::from(px[2]);
                let nr = (393 * r + 769 * g + 189 * b) / 1000;
                let ng = (349 * r + 686 * g + 168 * b) / 1000;
                let nb = (272 * r + 534 * g + 131 * b) / 1000;
                px[0] = clamp_u8(nr);
                px[1] = clamp_u8(ng);
                px[2] = clamp_u8(nb);
            }
            Image {
                width: img.width,
                height: img.height,
                data: out,
            }
        }
        Filter::GaussianBlur3 => convolve_rgba(img, 1, &[1, 2, 1], |sum| clamp_u8((sum + 8) >> 4)),
        Filter::GaussianBlur5 => {
            convolve_rgba(img, 2, &[1, 4, 6, 4, 1], |sum| clamp_u8((sum + 128) >> 8))
        }
        Filter::BoxBlur(radius) => apply_box_blur(img, radius),
        Filter::Sobel => apply_sobel(img),
    }
}

/// Applies a sequence of [`Filter`]s in order, returning the resulting [`Image`].
#[must_use]
pub fn pipeline(img: &Image, filters: &[Filter]) -> Image {
    let mut current = img.clone();
    for filter in filters {
        current = apply(&current, filter);
    }
    current
}

/// Parses a comma-separated filter pipeline specification.
///
/// Supported steps:
/// - `grayscale`
/// - `invert`
/// - `brightness:<-255..255>`
/// - `contrast:<0..4000>`
/// - `threshold:<0..255>`
/// - `sepia`
/// - `blur3`
/// - `blur5`
/// - `box:<1..16>`
/// - `sobel`
///
/// # Errors
///
/// - [`FilterError::TooLarge`] if `spec.len() > 1_024`.
/// - [`FilterError::PipelineTooLong`] if the pipeline contains more than 32 steps.
/// - [`FilterError::UnknownFilter`] if a filter name is not recognized.
/// - [`FilterError::InvalidParameter`] if a parameter is missing, unexpected, or out of range.
pub fn parse_pipeline(spec: &str) -> Result<Vec<Filter>, FilterError> {
    if spec.len() > MAX_PIPELINE_SPEC_BYTES {
        return Err(FilterError::TooLarge);
    }
    let trimmed = spec.trim();
    if trimmed.is_empty() {
        return Err(FilterError::InvalidParameter(
            "pipeline specification is empty".to_string(),
        ));
    }

    let parts: Vec<&str> = trimmed.split(',').collect();
    if parts.len() > MAX_PIPELINE_STEPS {
        return Err(FilterError::PipelineTooLong(parts.len()));
    }

    let mut filters = Vec::with_capacity(parts.len());
    for raw_part in parts {
        let token = raw_part.trim();
        if token.is_empty() {
            return Err(FilterError::InvalidParameter(
                "empty step in pipeline".to_string(),
            ));
        }
        let (name, param_opt) = match token.split_once(':') {
            Some((n, p)) => (n.trim(), Some(p.trim())),
            None => (token, None),
        };

        let filter = match name {
            "grayscale" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::Grayscale
            }
            "invert" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::Invert
            }
            "sepia" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::Sepia
            }
            "blur3" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::GaussianBlur3
            }
            "blur5" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::GaussianBlur5
            }
            "sobel" => {
                reject_unexpected_param(name, param_opt)?;
                Filter::Sobel
            }
            "brightness" => {
                let raw = require_param(name, param_opt)?;
                let val: i16 = raw
                    .parse()
                    .map_err(|_| FilterError::InvalidParameter(token.to_string()))?;
                if !(-255..=255).contains(&val) {
                    return Err(FilterError::InvalidParameter(token.to_string()));
                }
                Filter::Brightness(val)
            }
            "contrast" => {
                let raw = require_param(name, param_opt)?;
                let val: u16 = raw
                    .parse()
                    .map_err(|_| FilterError::InvalidParameter(token.to_string()))?;
                if val > 4000 {
                    return Err(FilterError::InvalidParameter(token.to_string()));
                }
                Filter::Contrast(val)
            }
            "threshold" => {
                let raw = require_param(name, param_opt)?;
                let val: u8 = raw
                    .parse()
                    .map_err(|_| FilterError::InvalidParameter(token.to_string()))?;
                Filter::Threshold(val)
            }
            "box" => {
                let raw = require_param(name, param_opt)?;
                let val: u8 = raw
                    .parse()
                    .map_err(|_| FilterError::InvalidParameter(token.to_string()))?;
                if !(1..=16).contains(&val) {
                    return Err(FilterError::InvalidParameter(token.to_string()));
                }
                Filter::BoxBlur(val)
            }
            other => return Err(FilterError::UnknownFilter(other.to_string())),
        };

        filters.push(filter);
    }

    Ok(filters)
}

fn reject_unexpected_param(name: &str, param_opt: Option<&str>) -> Result<(), FilterError> {
    if param_opt.is_some() {
        return Err(FilterError::InvalidParameter(format!(
            "{name} takes no parameter"
        )));
    }
    Ok(())
}

fn require_param<'a>(name: &str, param_opt: Option<&'a str>) -> Result<&'a str, FilterError> {
    match param_opt {
        Some(p) if !p.is_empty() => Ok(p),
        _ => Err(FilterError::InvalidParameter(format!(
            "{name} requires a parameter"
        ))),
    }
}
