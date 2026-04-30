//! Regenerate `assets/icon.ico` from `assets/icon.png`.
//!
//! Multi-resolution Windows icons (.ico) are PNG-encoded inside an
//! ICO container.  Run on demand whenever the source kangaroo PNG
//! changes:
//!
//!     cargo run --release --example make_ico
//!
//! No-op on macOS / Linux at build time — the Windows .exe icon
//! comes from `winresource` in `build.rs`, which reads this file.

use std::{fs::File, io::BufWriter};

const SIZES: &[u32] = &[16, 24, 32, 48, 64, 128, 256];

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = image::open("assets/icon.png")?.to_rgba8();
    let mut dir = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in SIZES {
        let resized = image::imageops::resize(
            &src,
            size,
            size,
            image::imageops::FilterType::Lanczos3,
        );
        let image = ico::IconImage::from_rgba_data(size, size, resized.into_raw());
        dir.add_entry(ico::IconDirEntry::encode(&image)?);
    }
    let out = BufWriter::new(File::create("assets/icon.ico")?);
    dir.write(out)?;
    println!("wrote assets/icon.ico ({} sizes)", SIZES.len());
    Ok(())
}
