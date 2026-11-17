//! Deterministic digest test suite shared between native and `wasm32-wasip1` targets.

use wasm_image_filters::{Filter, Image, apply};

const WIDTH: u32 = 64;
const HEIGHT: u32 = 48;
const BASE_SEED: u64 = 1;

fn xorshift64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    *state = x;
    x
}

fn seeded_images() -> [(&'static str, Image); 3] {
    let mut state = BASE_SEED;
    let names = ["img0", "img1", "img2"];
    names.map(|name| {
        let len = usize::try_from(WIDTH * HEIGHT * 4).expect("fits in usize");
        let mut buf = Vec::with_capacity(len);
        for _ in 0..len {
            let v = (xorshift64(&mut state) & 0xff) as u8;
            buf.push(v);
        }
        (
            name,
            Image::new(WIDTH, HEIGHT, buf).expect("valid 64x48 image"),
        )
    })
}

fn nine_filters() -> [(&'static str, Filter); 9] {
    [
        ("grayscale", Filter::Grayscale),
        ("invert", Filter::Invert),
        ("brightness", Filter::Brightness(40)),
        ("contrast", Filter::Contrast(1500)),
        ("threshold", Filter::Threshold(128)),
        ("sepia", Filter::Sepia),
        ("blur3", Filter::GaussianBlur3),
        ("blur5", Filter::GaussianBlur5),
        ("sobel", Filter::Sobel),
    ]
}

#[test]
fn print_digests() {
    use std::io::Write;
    let images = seeded_images();
    let filters = nine_filters();
    for (f_name, filter) in &filters {
        for (i_name, img) in &images {
            let out = apply(img, filter);
            let digest = out.digest();
            println!("digest {f_name} {i_name} 0x{digest:016x}");
        }
    }
    let _ = std::io::stdout().flush();
}
