//! Integration and property tests of the public API.

use proptest::prelude::*;

use wasm_image_filters::{Filter, FilterError, Image, apply, cli, parse_pipeline, pipeline, ppm};

fn single_pixel(r: u8, g: u8, b: u8, a: u8) -> Image {
    Image::new(1, 1, vec![r, g, b, a]).expect("valid 1x1 image")
}

#[test]
fn grayscale_exact_values_and_preserves_alpha() {
    let red = apply(&single_pixel(255, 0, 0, 255), &Filter::Grayscale);
    assert_eq!(red.data(), &[54, 54, 54, 255]);

    let green = apply(&single_pixel(0, 255, 0, 255), &Filter::Grayscale);
    assert_eq!(green.data(), &[182, 182, 182, 255]);

    let blue = apply(&single_pixel(0, 0, 255, 255), &Filter::Grayscale);
    assert_eq!(blue.data(), &[19, 19, 19, 255]);

    let white = apply(&single_pixel(255, 255, 255, 255), &Filter::Grayscale);
    assert_eq!(white.data(), &[255, 255, 255, 255]);
}

#[test]
fn point_filters_exact_values() {
    let inv = apply(&single_pixel(10, 20, 30, 128), &Filter::Invert);
    assert_eq!(inv.data(), &[245, 235, 225, 128]);

    let b_pos = apply(&single_pixel(230, 100, 0, 200), &Filter::Brightness(50));
    assert_eq!(b_pos.data(), &[255, 150, 50, 200]);

    let b_neg = apply(&single_pixel(30, 100, 255, 77), &Filter::Brightness(-50));
    assert_eq!(b_neg.data(), &[0, 50, 205, 77]);

    let contrast = apply(&single_pixel(200, 100, 128, 99), &Filter::Contrast(1500));
    assert_eq!(contrast.data(), &[236, 86, 128, 99]);

    let thresh_low = apply(&single_pixel(127, 127, 127, 42), &Filter::Threshold(128));
    assert_eq!(thresh_low.data(), &[0, 0, 0, 42]);

    let thresh_high = apply(&single_pixel(128, 128, 128, 42), &Filter::Threshold(128));
    assert_eq!(thresh_high.data(), &[255, 255, 255, 42]);

    let sepia_mid = apply(&single_pixel(100, 100, 100, 255), &Filter::Sepia);
    assert_eq!(sepia_mid.data(), &[135, 120, 93, 255]);

    let sepia_white = apply(&single_pixel(255, 255, 255, 255), &Filter::Sepia);
    assert_eq!(sepia_white.data(), &[255, 255, 238, 255]);
}

#[test]
fn gaussian_blur3_impulse_response() {
    let mut data = vec![0u8; 3 * 3 * 4];
    for px in data.as_chunks_mut::<4>().0 {
        px[3] = 255;
    }
    let center = 4 * 4;
    data[center] = 255;
    data[center + 1] = 255;
    data[center + 2] = 255;

    let img = Image::new(3, 3, data).expect("valid 3x3");
    let blurred = apply(&img, &Filter::GaussianBlur3);

    let expected_luma = [16u8, 32, 16, 32, 64, 32, 16, 32, 16];
    for (i, &want) in expected_luma.iter().enumerate() {
        let px = &blurred.data()[i * 4..(i + 1) * 4];
        assert_eq!(px, &[want, want, want, 255], "pixel {i}");
    }
}

#[test]
fn sobel_vertical_step_and_uniform() {
    let mut data = Vec::with_capacity(4 * 3 * 4);
    for _y in 0..3 {
        for x in 0..4 {
            let v = if x < 2 { 0u8 } else { 255u8 };
            data.extend_from_slice(&[v, v, v, 255]);
        }
    }
    let img = Image::new(4, 3, data).expect("valid 4x3");
    let edges = apply(&img, &Filter::Sobel);

    let expected_row = [0u8, 255, 255, 0];
    for y in 0..3 {
        for (x, &want) in expected_row.iter().enumerate() {
            let idx = (y * 4 + x) * 4;
            assert_eq!(
                &edges.data()[idx..idx + 4],
                &[want, want, want, 255],
                "at ({x}, {y})"
            );
        }
    }

    let uniform = Image::new(4, 3, vec![90u8; 4 * 3 * 4]).expect("valid uniform");
    let uniform_edges = apply(&uniform, &Filter::Sobel);
    for px in uniform_edges.data().as_chunks::<4>().0 {
        assert_eq!(px, &[0, 0, 0, 90]);
    }
}

#[test]
fn parse_pipeline_valid_and_invalid_cases() {
    let parsed = parse_pipeline("grayscale,blur3,threshold:128").expect("valid 3-step pipeline");
    assert_eq!(
        parsed,
        vec![
            Filter::Grayscale,
            Filter::GaussianBlur3,
            Filter::Threshold(128),
        ]
    );

    let all_steps = parse_pipeline(
        "grayscale,invert,brightness:-20,contrast:1200,threshold:100,sepia,blur3,blur5,box:2,sobel",
    )
    .expect("all steps valid");
    assert_eq!(all_steps.len(), 10);

    let err_unknown = parse_pipeline("sharpen").expect_err("unknown filter");
    assert_eq!(err_unknown.code(), "unknown_filter");

    let err_param = parse_pipeline("threshold:300").expect_err("invalid threshold");
    assert_eq!(err_param.code(), "invalid_parameter");

    let thirty_three = vec!["grayscale"; 33].join(",");
    let err_long = parse_pipeline(&thirty_three).expect_err("33 steps should fail");
    assert_eq!(err_long.code(), "pipeline_too_long");

    let over_1k = "a".repeat(1025);
    let err_huge = parse_pipeline(&over_1k).expect_err("too large spec");
    assert_eq!(err_huge.code(), "too_large");

    for bad in [
        "",
        "   ",
        "grayscale,",
        "grayscale:10",
        "brightness",
        "brightness:300",
        "brightness:-300",
        "contrast:5000",
        "contrast:abc",
        "box:0",
        "box:17",
        "box:xyz",
    ] {
        let err = parse_pipeline(bad).expect_err(bad);
        assert_eq!(err.code(), "invalid_parameter", "case {bad:?}");
    }
}

#[test]
fn image_validation_and_digest_stability() {
    let mismatch = Image::new(2, 2, vec![0; 15]).expect_err("15 != 16");
    assert_eq!(mismatch.code(), "size_mismatch");
    assert!(mismatch.to_string().contains("size_mismatch"));

    let zero_w = Image::new(0, 2, vec![]).expect_err("zero width");
    assert_eq!(zero_w.code(), "zero_dimension");

    let zero_h = Image::new(2, 0, vec![]).expect_err("zero height");
    assert_eq!(zero_h.code(), "zero_dimension");

    let too_large = Image::new(4097, 4096, vec![]).expect_err("exceeds 16_777_216 pixels");
    assert_eq!(too_large.code(), "too_large");

    let tiny = Image::new(1, 1, vec![0, 0, 0, 0]).expect("valid 1x1");
    assert_eq!(tiny.width(), 1);
    assert_eq!(tiny.height(), 1);
    assert_eq!(tiny.digest(), 9_087_969_665_827_829_253_u64);
    assert_eq!(
        tiny.digest(),
        Image::new(1, 1, vec![0, 0, 0, 0]).unwrap().digest()
    );
}

#[test]
fn ppm_codec_and_cli_execution() {
    let img = Image::new(2, 1, vec![255, 0, 0, 255, 0, 255, 0, 255]).expect("2x1");
    let encoded = ppm::encode_ppm(&img);
    let decoded = ppm::decode_ppm(&encoded).expect("decode valid ppm");
    assert_eq!(decoded, img);

    let with_comment = b"P6\n# comment line\n1 1\n255\n\x0a\x14\x1e";
    let comment_img = ppm::decode_ppm(with_comment).expect("ppm with comment");
    assert_eq!(comment_img.data(), &[10, 20, 30, 255]);

    for bad_ppm in [
        &b"P3\n1 1\n255\n\x00\x00\x00"[..],
        &b"P6\n1 1\n65535\n\x00\x00\x00"[..],
        &b"P6\n1 1\n255\n\x00\x00"[..],
        &b"P6\n"[..],
    ] {
        let err = ppm::decode_ppm(bad_ppm).expect_err("bad ppm");
        assert_eq!(err.code(), "invalid_ppm");
    }

    assert_eq!(
        ppm::decode_ppm(b"P6\n0 1\n255\n").unwrap_err().code(),
        "zero_dimension"
    );
    assert_eq!(
        ppm::decode_ppm(b"P6\n4097 4096\n255\n").unwrap_err().code(),
        "too_large"
    );

    let tmp_dir = std::env::temp_dir();
    let in_path = tmp_dir.join("wasm_image_filters_test_in.ppm");
    let out_path = tmp_dir.join("wasm_image_filters_test_out.ppm");
    std::fs::write(&in_path, &encoded).expect("write temp ppm");

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let args = vec![
        "apply".to_string(),
        in_path.to_string_lossy().into_owned(),
        "--filters".to_string(),
        "grayscale,invert".to_string(),
        "--out".to_string(),
        out_path.to_string_lossy().into_owned(),
    ];
    let code = cli::run(&args, &mut stdout, &mut stderr);
    assert_eq!(code, 0, "stderr: {}", String::from_utf8_lossy(&stderr));
    assert!(String::from_utf8_lossy(&stdout).contains("digest 0x"));

    let out_bytes = std::fs::read(&out_path).expect("read output ppm");
    let out_img = ppm::decode_ppm(&out_bytes).expect("decode output ppm");
    let expected = pipeline(&img, &[Filter::Grayscale, Filter::Invert]);
    assert_eq!(out_img, expected);

    let _ = std::fs::remove_file(&in_path);
    let _ = std::fs::remove_file(&out_path);

    // Help and error paths
    assert_eq!(
        cli::run(&["--help".to_string()], &mut stdout, &mut stderr),
        0
    );
    assert_eq!(cli::run(&[], &mut stdout, &mut stderr), 2);
    assert_eq!(
        cli::run(&["unknown".to_string()], &mut stdout, &mut stderr),
        2
    );
    assert_eq!(
        cli::run(
            &["apply".to_string(), "--bad-flag".to_string()],
            &mut stdout,
            &mut stderr
        ),
        2
    );
}

#[test]
fn error_formatting_truncates_long_messages() {
    let long_input = "x".repeat(300);
    let err = FilterError::UnknownFilter(long_input);
    assert!(err.to_string().len() <= 200);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn uniform_image_is_unchanged_by_every_blur(
        w in 1u32..=12,
        h in 1u32..=12,
        r in 0u8..=255,
        g in 0u8..=255,
        b in 0u8..=255,
        a in 0u8..=255,
        box_radius in 1u8..=4,
    ) {
        let count = usize::try_from(w * h).unwrap();
        let mut data = Vec::with_capacity(count * 4);
        for _ in 0..count {
            data.extend_from_slice(&[r, g, b, a]);
        }
        let img = Image::new(w, h, data).unwrap();
        prop_assert_eq!(apply(&img, &Filter::GaussianBlur3), img.clone());
        prop_assert_eq!(apply(&img, &Filter::GaussianBlur5), img.clone());
        prop_assert_eq!(apply(&img, &Filter::BoxBlur(box_radius)), img);
    }

    #[test]
    fn invert_twice_is_identity_and_alpha_preserved(
        w in 1u32..=8,
        h in 1u32..=8,
        seed in any::<u64>(),
        thresh in any::<u8>(),
    ) {
        let count = usize::try_from(w * h * 4).unwrap();
        let mut state = seed | 1;
        let mut data = Vec::with_capacity(count);
        for _ in 0..count {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            data.push((state & 0xff) as u8);
        }
        let img = Image::new(w, h, data).unwrap();
        let twice = pipeline(&img, &[Filter::Invert, Filter::Invert]);
        prop_assert_eq!(twice, img.clone());

        let thresholded = apply(&img, &Filter::Threshold(thresh));
        for (src_px, dst_px) in img.data().as_chunks::<4>().0.iter().zip(thresholded.data().as_chunks::<4>().0.iter()) {
            prop_assert!(dst_px[0] == 0 || dst_px[0] == 255);
            prop_assert_eq!(dst_px[0], dst_px[1]);
            prop_assert_eq!(dst_px[1], dst_px[2]);
            prop_assert_eq!(dst_px[3], src_px[3]);
        }
    }
}
