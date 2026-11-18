//! Binary entrypoint for `trysac-wasm-image-filters`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let code =
        wasm_image_filters::cli::run(&args[1..], &mut std::io::stdout(), &mut std::io::stderr());
    std::process::exit(code);
}
