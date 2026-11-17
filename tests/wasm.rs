//! CI-only WebAssembly conformance test comparing `wasm32-wasip1` under `wasmtime` with native digests.
#![cfg(feature = "conformance")]

use std::collections::BTreeMap;
use std::process::Command;

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

fn native_digests() -> BTreeMap<(String, String), String> {
    let mut state = BASE_SEED;
    let names = ["img0", "img1", "img2"];
    let images = names.map(|name| {
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
    });

    let filters = [
        ("grayscale", Filter::Grayscale),
        ("invert", Filter::Invert),
        ("brightness", Filter::Brightness(40)),
        ("contrast", Filter::Contrast(1500)),
        ("threshold", Filter::Threshold(128)),
        ("sepia", Filter::Sepia),
        ("blur3", Filter::GaussianBlur3),
        ("blur5", Filter::GaussianBlur5),
        ("sobel", Filter::Sobel),
    ];

    let mut map = BTreeMap::new();
    for (f_name, filter) in &filters {
        for (i_name, img) in &images {
            let out = apply(img, filter);
            map.insert(
                ((*f_name).to_string(), (*i_name).to_string()),
                format!("0x{:016x}", out.digest()),
            );
        }
    }
    map
}

/// Collects `digest <filter> <image> <hex>` records from the wasm test runner's stdout.
///
/// Under `wasm32-wasip1` libtest runs single-threaded, so with `--nocapture` it prints
/// `test print_digests ... ` without a newline before the test body runs. The first digest
/// record is therefore glued to that prefix, so the record is taken as the last four tokens
/// of a line whose fourth-from-last token is `digest`.
fn parse_digest_lines(stdout: &str) -> BTreeMap<(String, String), String> {
    let mut map = BTreeMap::new();
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if let [.., "digest", filter, image, hex] = parts.as_slice() {
            map.insert(
                ((*filter).to_string(), (*image).to_string()),
                (*hex).to_string(),
            );
        }
    }
    map
}

#[test]
fn parse_digest_lines_accepts_record_glued_to_libtest_prefix() {
    let stdout = "running 1 test\n\
                  test print_digests ... digest grayscale img0 0x00000000000000aa\n\
                  digest grayscale img1 0x00000000000000bb\n\
                  ok\n";
    let map = parse_digest_lines(stdout);
    assert_eq!(map.len(), 2);
    assert_eq!(
        map.get(&("grayscale".to_string(), "img0".to_string())),
        Some(&"0x00000000000000aa".to_string())
    );
    assert_eq!(
        map.get(&("grayscale".to_string(), "img1".to_string())),
        Some(&"0x00000000000000bb".to_string())
    );
}

fn has_wasmtime() -> bool {
    Command::new("wasmtime")
        .arg("--version")
        .output()
        .is_ok_and(|out| out.status.success())
}

#[test]
fn wasm32_wasip1_digests_match_native_exactly() {
    if !has_wasmtime() {
        eprintln!("skipping wasm32-wasip1 conformance test: wasmtime not found on PATH");
        return;
    }

    let target_dir = std::env::temp_dir().join("wasm-image-filters-wasm-target");
    let output = Command::new("cargo")
        .args([
            "test",
            "--target",
            "wasm32-wasip1",
            "--test",
            "digests",
            "--",
            "--nocapture",
            "--exact",
            "print_digests",
        ])
        .env("CARGO_TARGET_WASM32_WASIP1_RUNNER", "wasmtime")
        .env("CARGO_TARGET_DIR", &target_dir)
        .env_remove("RUSTFLAGS")
        .env_remove("CARGO_ENCODED_RUSTFLAGS")
        .env_remove("RUSTC_WRAPPER")
        .env_remove("RUSTC_WORKSPACE_WRAPPER")
        .output()
        .expect("failed to invoke cargo test --target wasm32-wasip1");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "wasm32-wasip1 test failed:\nstdout:\n{stdout}\nstderr:\n{stderr}"
    );

    let wasm_map = parse_digest_lines(&stdout);
    let expected = native_digests();
    assert_eq!(wasm_map.len(), 27, "expected 27 digest lines from wasm");
    assert_eq!(wasm_map, expected);
}
