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
use std::collections::HashMap;
use std::io::Read;
use std::sync::{Arc, Mutex, OnceLock};

use include_dir::{include_dir, Dir};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::{Icon, WindowBuilder},
};
use wry::{http::Response, WebViewBuilder};

mod games;
mod index_html;

/// Every HTML game file (and `makalaina_starting_positions.csv`) is
/// embedded directly into the binary at compile time.  Keeps the
/// release artefact a single executable on every platform.
static GAMES_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/assets/games");

/// Optional disk overlay loaded from `<data_dir>/Parados/games/`.
/// "Spiele aktualisieren" downloads fresh HTML from
/// `raw.githubusercontent.com/zdavatz/parados/main/<file>` into this
/// directory; the custom-protocol handler reads from the overlay
/// first, falls back to the embedded `GAMES_DIR`.  Same UX as the
/// iOS / Android ports' menu refresh.
type Overlay = Arc<Mutex<HashMap<String, Vec<u8>>>>;
static OVERLAY: OnceLock<Overlay> = OnceLock::new();

fn overlay() -> &'static Overlay {
    OVERLAY.get_or_init(|| Arc::new(Mutex::new(HashMap::new())))
}

/// `<data_dir>/Parados/games/`.  ~/Library/Application Support on macOS,
/// %APPDATA% on Windows, ~/.local/share on Linux.
fn games_cache_dir() -> Option<std::path::PathBuf> {
    Some(dirs::data_dir()?.join("Parados").join("games"))
}

/// Read every file in the games cache directory into the overlay.
/// Called once at startup so a previously-downloaded refresh
/// survives across launches.
fn load_overlay_from_disk() {
    let Some(dir) = games_cache_dir() else { return };
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let map = overlay();
    let mut guard = map.lock().expect("overlay lock");
    for entry in entries.flatten() {
        let p = entry.path();
        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            if let Ok(bytes) = std::fs::read(&p) {
                guard.insert(name.to_string(), bytes);
            }
        }
    }
}

/// Custom event the worker thread fires once "Spiele aktualisieren"
/// finishes — picked up by the main event loop, forwarded into the
/// webview as a `parados_update_done` JS callback.
#[derive(Debug, Clone)]
enum UserEvent {
    UpdateDone { updated: usize, total: usize, error: Option<String> },
}

/// Kangaroo logo used in the top-right of the menu page (and as the
/// window icon).  Same JPEG as the iOS `kangy.imageset` — keeps the
/// three ports visually identical.
static KANGY_JPG: &[u8] = include_bytes!("../assets/kangy.jpg");

/// Larger-resolution PNG used for the window icon.  The Android
/// 512×512 Play Store icon is the highest-resolution clean source we
/// have; decoding once at startup is cheap.
static ICON_PNG: &[u8] = include_bytes!("../assets/icon.png");

/// Auto-dismiss the rules modal on every game when `--screenshot` is
/// passed.  Lets `screenshots/macos/capture.sh` produce gameplay
/// screenshots instead of "rules" screenshots.  Strategy: try the
/// well-known global `closeRules()` / `closeModal()` functions every
/// game defines, then fall back to clicking any modal close-button,
/// then to brute-force `display:none` on any visible modal.
const RULES_DISMISS_JS: &str = r#"
(function () {
    function dismiss() {
        try { if (typeof closeRules === 'function') { closeRules(); return; } } catch (e) {}
        try { if (typeof closeModal === 'function') { closeModal(); return; } } catch (e) {}
        var modals = document.querySelectorAll('#rulesModal, .rules-modal, .modal');
        modals.forEach(function (m) {
            if (getComputedStyle(m).display !== 'none') {
                var close = m.querySelector('.close, [onclick*="closeRules"], [onclick*="closeModal"]');
                if (close) close.click();
                else m.style.display = 'none';
            }
        });
    }
    function schedule() { setTimeout(dismiss, 350); setTimeout(dismiss, 900); }
    if (document.readyState !== 'loading') schedule();
    else document.addEventListener('DOMContentLoaded', schedule);
})();
"#;

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
    // Optional `--url <url>` arg deep-links into a specific game on
    // launch.  Used by `screenshots/macos/capture.sh` to pre-load each
    // game and grab a screenshot without scripting button clicks; also
    // useful for ad-hoc smoke tests.
    let args: Vec<String> = std::env::args().collect();
    let mut initial_url = "parados://localhost/".to_string();
    let mut screenshot_mode = false;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--url" && i + 1 < args.len() {
            initial_url = args[i + 1].clone();
            i += 2;
        } else if args[i] == "--screenshot" {
            screenshot_mode = true;
            i += 1;
        } else {
            i += 1;
        }
    }

    load_overlay_from_disk();

    let event_loop: tao::event_loop::EventLoop<UserEvent> =
        EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let icon = decode_icon(ICON_PNG);
    // Show the running version after the app name in the title bar —
    // identical wording on every platform (tao maps with_title to
    // NSWindow.title on macOS, the WS_CAPTION text on Windows, and
    // the X11/Wayland window title on Linux).
    let title = format!("Parados {}", env!("CARGO_PKG_VERSION"));
    let mut window_builder = WindowBuilder::new()
        .with_title(&title)
        .with_inner_size(tao::dpi::LogicalSize::new(960.0, 720.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(480.0, 480.0));
    if let Some(ref icon) = icon {
        window_builder = window_builder.with_window_icon(Some(icon.clone()));
    }
    let window = window_builder.build(&event_loop).expect("window");

    // macOS Dock icon (no-op on Linux / Windows — those use the
    // `with_window_icon` we just set above).
    set_macos_dock_icon();

    // Build the init-script chain.  Always include the back-to-menu
    // button; in `--screenshot` mode also inject the rules-dismiss
    // helper so screenshots show gameplay, not the rules modal.
    let init_script = if screenshot_mode {
        format!("{}\n{}", BACK_BUTTON_JS, RULES_DISMISS_JS)
    } else {
        BACK_BUTTON_JS.to_string()
    };

    let webview = WebViewBuilder::new(&window)
        .with_url(&initial_url)
        .with_custom_protocol("parados".into(), move |request| {
            handle_request(request)
        })
        // Any http(s):// navigation triggered from inside a game (the
        // `Share on WhatsApp` button in particular — that calls
        // `window.location.href = "https://wa.me/?text=..."`) needs to
        // hand off to the user's default browser, which on macOS routes
        // wa.me URLs into the native WhatsApp app via its URL handler,
        // and on Windows / Linux into the browser tab the user expects.
        // Without this, wry would load wa.me inside our embedded
        // webview, which fails because PeerJS / WhatsApp Web require
        // browser features we don't expose.
        .with_navigation_handler(|url: String| {
            if url.starts_with("http://") || url.starts_with("https://") {
                if let Err(e) = open::that(&url) {
                    eprintln!("parados: failed to open {url} externally: {e}");
                }
                return false; // cancel in-webview navigation
            }
            true // allow parados:// internal navigation
        })
        // Same logic for `window.open(url, '_blank')` calls — those go
        // through wry's new-window handler.  `shareOnWhatsApp()` tries
        // `window.open` first, falls back to `window.location.href`, so
        // we have to catch both sites.
        .with_new_window_req_handler(|url: String| {
            if url.starts_with("http://") || url.starts_with("https://") {
                if let Err(e) = open::that(&url) {
                    eprintln!("parados: failed to open {url} externally: {e}");
                }
                return false; // don't open a new wry window
            }
            true
        })
        .with_initialization_script(&init_script)
        .with_ipc_handler({
            let proxy = proxy.clone();
            move |request| {
                let body = request.body();
                // `open-external:<url>` — remote-multiplayer variants +
                // the menu footer link, routed to the default browser.
                if let Some(url) = body.strip_prefix("open-external:") {
                    if let Err(e) = open::that(url) {
                        eprintln!("parados: failed to open {url} in default browser: {e}");
                    }
                    return;
                }
                // `update-games` — download fresh HTML from GitHub
                // into the overlay cache.  Spawn a worker thread so
                // the IPC callback returns immediately; signal
                // completion via the EventLoopProxy so the main
                // thread can drive the webview's JS-side toast.
                if body == "update-games" {
                    let proxy = proxy.clone();
                    std::thread::spawn(move || {
                        let outcome = update_games_blocking();
                        let _ = proxy.send_event(outcome);
                    });
                    return;
                }
                eprintln!("parados: ignoring unknown IPC message: {body}");
            }
        })
        .build()?;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            Event::UserEvent(UserEvent::UpdateDone { updated, total, error }) => {
                // Forward to the webview's JS-side toast handler.
                let err = error.unwrap_or_default()
                    .replace('\\', "\\\\").replace('"', "\\\"");
                let js = format!(
                    "if (typeof window.parados_update_done === 'function') {{ \
                        window.parados_update_done({updated}, {total}, \"{err}\"); \
                     }}"
                );
                let _ = webview.evaluate_script(&js);
            }
            _ => {}
        }
    });
}

/// Blocking implementation of "Spiele aktualisieren".  Downloads
/// every entry in `games::ALL_FILENAMES` from
/// `raw.githubusercontent.com/zdavatz/parados/main/<file>`, writes
/// them under `<data_dir>/Parados/games/`, and updates the in-memory
/// overlay so subsequent navigations see the fresh content.  Returns
/// a `UserEvent::UpdateDone` describing the outcome.
fn update_games_blocking() -> UserEvent {
    const BASE: &str = "https://raw.githubusercontent.com/zdavatz/parados/main/";
    let total = games::ALL_FILENAMES.len();
    let dir = match games_cache_dir() {
        Some(d) => d,
        None => return UserEvent::UpdateDone {
            updated: 0, total, error: Some("no data directory".into())
        },
    };
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return UserEvent::UpdateDone {
            updated: 0, total, error: Some(format!("mkdir failed: {e}"))
        };
    }

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(30))
        .build();

    let mut updated = 0;
    let mut last_error = None;
    for name in games::ALL_FILENAMES {
        let url = format!("{BASE}{name}");
        match agent.get(&url).call() {
            Ok(resp) if resp.status() == 200 => {
                let mut bytes = Vec::new();
                if let Err(e) = resp.into_reader().read_to_end(&mut bytes) {
                    last_error = Some(format!("{name}: read body failed: {e}"));
                    continue;
                }
                let target = dir.join(name);
                if let Err(e) = std::fs::write(&target, &bytes) {
                    last_error = Some(format!("{name}: write failed: {e}"));
                    continue;
                }
                if let Ok(mut guard) = overlay().lock() {
                    guard.insert((*name).to_string(), bytes);
                    updated += 1;
                }
            }
            Ok(resp) => {
                last_error = Some(format!("{name}: HTTP {}", resp.status()));
            }
            Err(e) => {
                last_error = Some(format!("{name}: {e}"));
            }
        }
    }
    UserEvent::UpdateDone { updated, total, error: last_error }
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

    // Game HTML / CSV — `parados://localhost/games/<file>`.  Check
    // the overlay (downloaded refreshes) first, fall back to the
    // embedded bundle.
    if let Some(rest) = trimmed.strip_prefix("games/") {
        if let Ok(guard) = overlay().lock() {
            if let Some(bytes) = guard.get(rest) {
                return ok(bytes.clone(), guess_mime(rest));
            }
        }
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

/// macOS-only: set the running NSApplication's Dock icon to
/// `assets/icon.png`.  `WindowBuilder::with_window_icon` does the
/// right thing on Linux (X11/Wayland) and Windows (taskbar +
/// title-bar minimap), but on macOS the Dock icon comes from
/// `NSApplication.applicationIconImage`, which `tao` does not
/// expose.  When the app is launched from a properly bundled
/// `.app` the Dock reads the icon from `Contents/Resources/icon.icns`
/// and this call is redundant; when launched from the bare release
/// binary (`cargo run`, `target/release/parados`) it's the only
/// thing that puts the kangaroo in the Dock.
#[cfg(target_os = "macos")]
#[allow(deprecated)]   // the cocoa crate is deprecated in favour of objc2-*; keep
                       // using it because tao still pins cocoa transitively, so
                       // adding objc2-app-kit would balloon the dep graph for a
                       // five-line call site.
fn set_macos_dock_icon() {
    use cocoa::appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicy, NSImage,
    };
    use cocoa::base::{id, nil};
    use cocoa::foundation::NSData;
    unsafe {
        let app: id = NSApp();

        // Without `Regular` activation policy macOS keeps the unbundled
        // `target/release/parados` binary in the generic-exec mode that
        // shows the black "exec" tile in the Dock instead of honouring
        // `setApplicationIconImage:`.  Forcing Regular makes the app
        // appear in the Dock + Cmd-Tab as a normal GUI app, which is
        // the prerequisite for the icon-image override below to stick.
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );

        let data: id = NSData::dataWithBytes_length_(
            nil,
            ICON_PNG.as_ptr() as *const std::ffi::c_void,
            ICON_PNG.len() as u64,
        );
        let image: id = NSImage::alloc(nil);
        let image: id = NSImage::initWithData_(image, data);
        if image != nil {
            app.setApplicationIconImage_(image);
        } else {
            eprintln!("parados: NSImage::initWithData_ returned nil — Dock icon not set");
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn set_macos_dock_icon() {}
