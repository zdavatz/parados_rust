//! Parados desktop GUI — embeds the seven Think Ahead board games
//! in a single native window via `tao` (window) + `wry` (webview).
//!
//! Rough flow:
//!
//! 1. `tao` opens a window with the kangaroo PNG decoded into a
//!    platform-native icon (Dock / taskbar / GNOME Activities).
//! 2. `wry` mounts a webview with a `parados://` custom protocol
//!    handler that serves *embedded* resources — the menu HTML
//!    rendered on the fly from `index_html::render`, the seven game
//!    HTML files baked in via `include_dir!`, and the kangaroo logo.
//! 3. The menu page links to `parados://localhost/games/<file>` for
//!    local play and emits `open-external:<https url>` IPC messages
//!    for the three remote-multiplayer variants, which we route to
//!    the user's default browser via the `open` crate.
//! 4. A pinned-overlay back button (HTML, injected at document start
//!    on every page that *isn't* the menu) returns to the menu —
//!    same UX as the auto-hiding back FAB on the Android / iOS
//!    ports, just always-visible on desktop where screen real-estate
//!    is plentiful.

use std::borrow::Cow;

use include_dir::{include_dir, Dir};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::{Icon, WindowBuilder},
};
use wry::{http::Response, WebViewBuilder};

mod games;
mod index_html;

/// Every HTML game file (and `makalaina_starting_positions.csv`) is
/// embedded directly into the binary at compile time.  Keeps the
/// release artefact a single executable on every platform.
static GAMES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/games");

/// Kangaroo logo used in the top-right of the menu page (and as the
/// window icon).  Same JPEG as the iOS `kangy.imageset` — keeps the
/// three ports visually identical.
static KANGY_JPG: &[u8] = include_bytes!("../assets/kangy.jpg");

/// Larger-resolution PNG used for the window icon.  The Android
/// 512×512 Play Store icon is the highest-resolution clean source we
/// have; decoding once at startup is cheap.
static ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Injected into every *game* page (not the menu) at document start.
/// Renders an always-on-screen "← Menu" pill in the top-left so the
/// user can leave a game without keyboard shortcuts.  We deliberately
/// don't auto-hide the way the iOS / Android ports do — desktop
/// users have a mouse, and the FAB is small enough to be unobtrusive.
const BACK_BUTTON_JS: &str = r#"
(function () {
    if (window.location.pathname === '/' || window.location.pathname === '/index.html') {
        return; // menu page — no back button needed
    }
    function installBackButton() {
        if (document.getElementById('__parados_back')) return;
        var btn = document.createElement('button');
        btn.id = '__parados_back';
        btn.textContent = '← Menu';
        btn.style.cssText = [
            'position:fixed','top:12px','left:12px','z-index:2147483647',
            'background:#37474f','color:#ffd700','border:none','border-radius:18px',
            'padding:8px 14px','font:600 13px -apple-system,BlinkMacSystemFont,Segoe UI,Roboto,sans-serif',
            'box-shadow:0 2px 6px rgba(0,0,0,0.4)','cursor:pointer','opacity:0.85'
        ].join(';');
        btn.onmouseenter = function () { btn.style.opacity = '1.0'; };
        btn.onmouseleave = function () { btn.style.opacity = '0.85'; };
        btn.onclick = function () { window.location.href = 'parados://localhost/'; };
        document.body.appendChild(btn);
    }
    if (document.body) installBackButton();
    else document.addEventListener('DOMContentLoaded', installBackButton);
})();
"#;

fn main() -> wry::Result<()> {
    let event_loop = EventLoop::new();

    let icon = decode_icon(ICON_PNG);
    let mut window_builder = WindowBuilder::new()
        .with_title("Parados")
        .with_inner_size(tao::dpi::LogicalSize::new(960.0, 720.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(480.0, 480.0));
    if let Some(ref icon) = icon {
        window_builder = window_builder.with_window_icon(Some(icon.clone()));
    }
    let window = window_builder.build(&event_loop).expect("window");

    let webview = WebViewBuilder::new(&window)
        .with_url("parados://localhost/")
        .with_custom_protocol("parados".into(), move |request| {
            handle_request(request)
        })
        .with_initialization_script(BACK_BUTTON_JS)
        .with_ipc_handler(|request| {
            // Single supported message: `open-external:<url>` from the
            // menu page's footer / remote-multiplayer variants.  The
            // iOS / Android ports do this with `UIApplication.open` /
            // `Intent.ACTION_VIEW`; on desktop we hand off to the
            // user's default browser via `open`.
            let body = request.body();
            if let Some(url) = body.strip_prefix("open-external:") {
                if let Err(e) = open::that(url) {
                    eprintln!("parados: failed to open {url} in default browser: {e}");
                }
            }
        })
        .build()?;
    let _ = webview; // kept alive in the event loop closure

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}

/// Resolve a `parados://` URL against the embedded resources and
/// return the response.  Never returns `Err` so the webview always
/// gets *some* page back — failure modes show up as a 404 inside
/// the webview, which is what we want.
fn handle_request(
    request: wry::http::Request<Vec<u8>>,
) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let trimmed = path.trim_start_matches('/');

    // Menu page (the `/` and `/index.html` aliases).
    if trimmed.is_empty() || trimmed == "index.html" {
        let body = index_html::render();
        return ok(body.into_bytes(), "text/html; charset=utf-8");
    }

    // Game HTML / CSV — `parados://localhost/games/<file>`.
    if let Some(rest) = trimmed.strip_prefix("games/") {
        if let Some(file) = GAMES_DIR.get_file(rest) {
            return ok(file.contents().to_vec(), guess_mime(rest));
        }
    }

    // Bundled assets — currently just the kangaroo logo.
    if trimmed == "assets/kangy.jpg" {
        return ok(KANGY_JPG.to_vec(), "image/jpeg");
    }
    if trimmed == "assets/icon.png" {
        return ok(ICON_PNG.to_vec(), "image/png");
    }

    not_found(trimmed)
}

fn ok(body: Vec<u8>, mime: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(200)
        .header("Content-Type", mime)
        .header("Cache-Control", "no-store")
        .body(Cow::Owned(body))
        .expect("response")
}

fn not_found(path: &str) -> Response<Cow<'static, [u8]>> {
    Response::builder()
        .status(404)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Cow::Owned(
            format!("<h1>404</h1><p>Not found: {path}</p>").into_bytes(),
        ))
        .expect("404 response")
}

fn guess_mime(filename: &str) -> &'static str {
    match filename.rsplit('.').next().unwrap_or("") {
        "html" | "htm" => "text/html; charset=utf-8",
        "css"          => "text/css; charset=utf-8",
        "js"           => "application/javascript; charset=utf-8",
        "json"         => "application/json; charset=utf-8",
        "csv"          => "text/csv; charset=utf-8",
        "svg"          => "image/svg+xml",
        "png"          => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif"          => "image/gif",
        "webp"         => "image/webp",
        _              => "application/octet-stream",
    }
}

/// Decode `assets/icon.png` into a `tao::window::Icon`.  Returns
/// `None` and prints a warning if anything goes wrong — a missing
/// icon should never block app launch.
fn decode_icon(bytes: &[u8]) -> Option<Icon> {
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (w, h) = img.dimensions();
    Icon::from_rgba(img.into_raw(), w, h).ok()
}
