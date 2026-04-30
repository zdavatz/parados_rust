//! Apply Apple-style rounded corners to `assets/icon.png` (in-place).
//!
//! Why a separate one-shot example instead of doing it at runtime:
//! macOS `setApplicationIconImage:` and tao's `with_window_icon`
//! both render whatever bytes we hand them as-is — no mask is
//! applied automatically when the binary isn't bundled in a `.app`.
//! Pre-baking the squircle into the source PNG gives us a rounded
//! Dock icon on every launch (cargo run, target/release/parados,
//! workflow Linux tarballs, Windows .exe via `assets/icon.ico`,
//! and the Microsoft Store tiles produced by `sips` in CI).
//!
//! Re-run after replacing the source kangaroo image:
//!     cargo run --release --example round_icon
//!     cargo run --release --example make_ico   # rebuilds icon.ico

use image::{ImageBuffer, Rgba};

const RADIUS_PCT: f32 = 0.225;   // matches the Apple "squircle" curvature ~22.5%

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let src = image::open("assets/icon.png")?.to_rgba8();
    let (w, h) = src.dimensions();
    let r = (w.min(h) as f32 * RADIUS_PCT) as u32;

    let mut out: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            // Distance from the corner inflection — 0 inside the
            // straight edges, increasing as we walk into a corner.
            let dx = if x < r {
                (r - 1 - x) as f32
            } else if x >= w - r {
                (x - (w - r)) as f32
            } else {
                0.0
            };
            let dy = if y < r {
                (r - 1 - y) as f32
            } else if y >= h - r {
                (y - (h - r)) as f32
            } else {
                0.0
            };

            // Outside any corner → fully opaque pixel from source.
            // Inside a corner → mask by quarter-circle distance, with
            // a 1-pixel anti-aliased edge so the curve doesn't
            // pixelate.
            let alpha_mult = if dx > 0.0 && dy > 0.0 {
                let dist = (dx * dx + dy * dy).sqrt();
                if dist > r as f32 {
                    0.0
                } else if dist > (r as f32 - 1.0) {
                    r as f32 - dist
                } else {
                    1.0
                }
            } else {
                1.0
            };

            let p = src.get_pixel(x, y);
            let new_alpha = (p[3] as f32 * alpha_mult).clamp(0.0, 255.0) as u8;
            out.put_pixel(x, y, Rgba([p[0], p[1], p[2], new_alpha]));
        }
    }

    out.save("assets/icon.png")?;
    println!(
        "rounded {w}×{h} (radius {r} px = {pct:.1}%) → assets/icon.png",
        pct = RADIUS_PCT * 100.0,
    );
    Ok(())
}
