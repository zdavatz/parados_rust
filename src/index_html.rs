//! Renders the game-list "menu" page as a single HTML document.
//! Mirrors the SwiftUI / Compose layout pixel-for-pixel:
//!
//! - background `#263238`, cards `#37474F`, gold accent `#FFD700`
//! - 5-color cycling buttons (green / blue / orange / red / purple)
//! - kangaroo logo in the top-right of the header
//!
//! The page is served from the `parados://localhost/` custom protocol;
//! the `Play` / variant buttons navigate to `parados://localhost/games/<file>`,
//! which loads the corresponding embedded HTML game.  Remote-multiplayer
//! variants carry a `data-external` attribute with their `https://` URL
//! and are routed back to native code via `window.ipc.postMessage` so
//! they open in the user's default browser.

use crate::games::GAMES;

/// Render the menu page.  Pure function — no IO, no globals — so the
/// custom-protocol handler can call it on every navigation to `/`.
pub fn render() -> String {
    let mut cards = String::new();
    for game in GAMES {
        cards.push_str(&render_card(game));
    }

    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>Parados</title>
<style>
  *, *::before, *::after {{ box-sizing: border-box; }}
  html, body {{
    margin: 0;
    padding: 0;
    background: #263238;
    color: #cfd8dc;
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
    min-height: 100vh;
  }}
  header {{
    position: sticky;
    top: 0;
    z-index: 10;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 20px;
    background: #263238;
    border-bottom: 1px solid #1c272c;
  }}
  header h1 {{
    margin: 0;
    font-size: 20px;
    font-weight: 700;
    letter-spacing: 2px;
    color: #ffd700;
  }}
  header button.kangy {{
    appearance: none;
    border: none;
    padding: 0;
    margin: 0;
    cursor: pointer;
    background: transparent;
    line-height: 0;
    position: relative;
  }}
  header button.kangy img.logo {{
    width: 40px;
    height: 40px;
    border-radius: 8px;
    object-fit: cover;
    background: #f5f0e8;
    transition: transform 0.15s ease, opacity 0.2s ease;
  }}
  header button.kangy:hover img.logo {{ transform: scale(1.05); }}
  header button.kangy.updating img.logo {{
    opacity: 0.5;
    animation: kangy-spin 0.9s linear infinite;
  }}
  @keyframes kangy-spin {{
    from {{ transform: rotate(0deg); }}
    to   {{ transform: rotate(360deg); }}
  }}
  #toast {{
    position: fixed;
    left: 50%;
    bottom: 24px;
    transform: translateX(-50%) translateY(40px);
    background: #37474f;
    color: #ffd700;
    padding: 10px 18px;
    border-radius: 24px;
    font-size: 14px;
    font-weight: 600;
    box-shadow: 0 4px 14px rgba(0,0,0,0.5);
    opacity: 0;
    pointer-events: none;
    transition: transform 0.25s ease, opacity 0.25s ease;
    z-index: 100;
  }}
  #toast.show {{
    opacity: 1;
    transform: translateX(-50%) translateY(0);
  }}
  main {{
    max-width: 720px;
    margin: 0 auto;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }}
  .card {{
    background: #37474f;
    border-radius: 12px;
    padding: 16px;
  }}
  .card .title {{
    font-size: 18px;
    font-weight: 700;
    color: #ffd700;
    margin: 0 0 6px;
  }}
  .card .players {{
    font-size: 13px;
    color: #90a4ae;
    margin: 0 0 8px;
  }}
  .card .description {{
    font-size: 14px;
    color: #cfd8dc;
    line-height: 1.45;
    margin: 0 0 12px;
  }}
  .buttons {{
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }}
  .buttons .btn {{
    flex: 1 1 0;
    min-width: 80px;
    border: none;
    border-radius: 20px;
    padding: 10px 14px;
    font-size: 14px;
    font-weight: 600;
    color: white;
    cursor: pointer;
    text-align: center;
  }}
  .btn[data-color="0"] {{ background: #43a047; }}
  .btn[data-color="1"] {{ background: #1e88e5; }}
  .btn[data-color="2"] {{ background: #ff9800; color: #263238; }}
  .btn[data-color="3"] {{ background: #e53935; }}
  .btn[data-color="4"] {{ background: #8e24aa; }}
  .btn:hover  {{ filter: brightness(1.10); }}
  .btn:active {{ filter: brightness(0.92); }}
  footer {{
    text-align: center;
    padding: 24px 16px 32px;
    color: #607d8b;
    font-size: 12px;
  }}
  footer a {{ color: #90a4ae; text-decoration: none; }}
</style>
</head>
<body>
  <header>
    <h1>PARADOS</h1>
    <button class="kangy" id="updateBtn" title="Spiele aktualisieren">
      <img class="logo" src="parados://localhost/assets/kangy.jpg" alt="Parados">
    </button>
  </header>
  <div id="toast"></div>
  <main>
{cards}  </main>
  <footer>
    Parados — Think Ahead Games · <a href="https://game.ywesee.com/parados/" data-external="https://game.ywesee.com/parados/">game.ywesee.com/parados</a>
  </footer>
<script>
  // Route every "Play / variant" click through a single delegated
  // handler.  Local games navigate inside the webview; remote
  // (PeerJS / WebRTC) games are sent back to Rust as
  // `open-external:<url>` IPC messages so they open in the user's
  // default browser — same UX as iOS / Android.
  document.addEventListener('click', function(ev) {{
    const el = ev.target.closest('[data-href], [data-external]');
    if (!el) return;
    ev.preventDefault();
    const ext = el.getAttribute('data-external');
    if (ext) {{
      try {{ window.ipc.postMessage('open-external:' + ext); }}
      catch (e) {{ window.location.href = ext; }}
      return;
    }}
    const href = el.getAttribute('data-href');
    if (href) {{ window.location.href = href; }}
  }});

  // "Spiele aktualisieren" — kangaroo top-right kicks off a
  // background download of every game HTML from
  // raw.githubusercontent.com/zdavatz/parados/main/.  Same UX as the
  // iOS Menu / Android toolbar.  Rust's worker thread fires
  // `window.parados_update_done(updated, total, error)` when done.
  const updateBtn = document.getElementById('updateBtn');
  const toast = document.getElementById('toast');
  let updateBusy = false;

  function showToast(msg, ms) {{
    toast.textContent = msg;
    toast.classList.add('show');
    clearTimeout(showToast._t);
    showToast._t = setTimeout(function () {{
      toast.classList.remove('show');
    }}, ms || 3000);
  }}

  updateBtn.addEventListener('click', function () {{
    if (updateBusy) return;
    updateBusy = true;
    updateBtn.classList.add('updating');
    showToast('Spiele werden aktualisiert…', 30000);
    try {{ window.ipc.postMessage('update-games'); }}
    catch (e) {{
      updateBusy = false;
      updateBtn.classList.remove('updating');
      showToast('Update fehlgeschlagen: IPC nicht verfügbar', 4000);
    }}
  }});

  window.parados_update_done = function (updated, total, error) {{
    updateBusy = false;
    updateBtn.classList.remove('updating');
    if (error && updated === 0) {{
      showToast('Update fehlgeschlagen: ' + error, 5000);
    }} else if (updated === total) {{
      showToast(updated + ' Spiele aktualisiert');
    }} else if (updated > 0) {{
      showToast(updated + ' / ' + total + ' aktualisiert (' + (error || '?') + ')', 5000);
    }} else {{
      showToast('Keine Updates verfügbar');
    }}
  }};
</script>
</body>
</html>
"##
    )
}

fn render_card(game: &crate::games::Game) -> String {
    let title       = html_escape::encode_text(game.title);
    let players     = html_escape::encode_text(game.players);
    let description = html_escape::encode_text(game.description);

    let mut buttons = String::new();
    if game.variants.is_empty() {
        buttons.push_str(&render_button(0, "Play", game.filename, None));
    } else {
        for (i, v) in game.variants.iter().enumerate() {
            buttons.push_str(&render_button(i, v.label, v.filename, v.url));
        }
    }

    format!(
        "    <div class=\"card\">\n\
             \x20     <h2 class=\"title\">{title}</h2>\n\
             \x20     <p class=\"players\">{players}</p>\n\
             \x20     <p class=\"description\">{description}</p>\n\
             \x20     <div class=\"buttons\">{buttons}</div>\n\
             \x20   </div>\n",
    )
}

fn render_button(idx: usize, label: &str, filename: &str, url: Option<&str>) -> String {
    let label = html_escape::encode_text(label);
    let color = idx % 5;
    match url {
        Some(u) => format!(
            r#"<button class="btn" data-color="{color}" data-external="{url}">{label}</button>"#,
            url = html_escape::encode_quoted_attribute(u),
        ),
        None => format!(
            r#"<button class="btn" data-color="{color}" data-href="parados://localhost/games/{file}">{label}</button>"#,
            file = html_escape::encode_quoted_attribute(filename),
        ),
    }
}
