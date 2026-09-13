//! Decode QR payloads from image files — a diagnostic with no network access.
//!
//!   cargo run -p mhy-qrscanner-qr --example decode_image -- frame-01.png frame-02.png …
//!
//! Prints one line per file plus a success count, which is how the frame-to-frame
//! decode rate of an animated/occluded QR is measured.

fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    if paths.is_empty() {
        eprintln!("usage: decode_image <image> [more images…]");
        std::process::exit(2);
    }
    let mut ok = 0usize;
    for path in &paths {
        match mhy_qrscanner_qr::decode_qr_file(std::path::Path::new(path)) {
            Ok(payload) => {
                ok += 1;
                println!("{path}: ok len={}", payload.len());
            }
            Err(e) => println!("{path}: no decode ({e})"),
        }
    }
    println!("decoded {ok}/{}", paths.len());
}
