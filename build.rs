//! Build script — embeds `assets/icon.ico` into the Windows .exe via
//! `winresource`.  No-op on Linux / macOS; those platforms handle
//! the icon through the .desktop file (Linux) or the .icns inside
//! the .app bundle (macOS), both of which are produced by the
//! release workflow rather than by `cargo build`.

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=build.rs");

    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=winresource icon embed failed: {e}");
        }
    }
}
