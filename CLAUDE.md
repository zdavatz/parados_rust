# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Rust desktop port of the Parados Think Ahead board games. Same seven HTML games shipped by
`parados_ios` (SwiftUI / WKWebView) and `parados_android` (RecyclerView / WebView), now
running cross-platform on Linux / macOS / Windows via `tao` (window) + `wry` (webview).
GPL-3.0.

Bundle ID: `com.ywesee.parados` · Microsoft Store reservation
`yweseeGmbH.Parados-ThinkAhead` (Store-ID `9N7RTWZQQ0K7`) · App Store Connect
record `6760842713` (Universal Purchase: same bundle ID across iOS + macOS).
Asset set is identical to the iOS / Android editions so all three stores
list the same product family.

## Architecture

Single binary (`parados`) — no library, no helper crates. Runtime structure:

- `src/main.rs` — opens a `tao` window with the kangaroo PNG decoded into a native icon,
  mounts a `wry` webview pointing at `parados://localhost/`. The custom-protocol handler
  serves the menu page (rendered by `index_html::render`) and every `assets/games/*.html`
  file (embedded via `include_dir!`). The IPC handler accepts two messages:
  - `open-external:<url>` — routes the three remote-multiplayer variants and the menu
    footer to the user's default browser via the `open` crate.
  - `update-games` — spawns a worker thread that downloads every entry in
    `games::ALL_FILENAMES` from `raw.githubusercontent.com/zdavatz/parados/main/<file>`
    via `ureq`, writes them under `<data_dir>/Parados/games/` (XDG-resolved per-platform
    via the `dirs` crate), and updates an in-memory `OVERLAY` so the next navigation
    serves fresh content. Completion is signalled to the main event loop through an
    `EventLoopProxy<UserEvent>` which evaluates a JS callback (`window.parados_update_done`)
    on the menu page to drive the spinner + toast UI. Mirrors the iOS Menu / Android
    toolbar "Spiele aktualisieren" UX.
  - The custom-protocol handler checks `OVERLAY` before falling back to the embedded
    `GAMES_DIR`, so refreshed games override bundled ones. On startup the overlay is
    re-loaded from disk so refreshes persist across launches.
  - A `--url <parados://...>` CLI arg deep-links into a specific game (used by
    `screenshots/macos/capture.sh`); a `--screenshot` CLI arg additionally injects
    `RULES_DISMISS_JS` to auto-close every game's rules modal so screenshots show
    actual gameplay.
- `src/games.rs` — direct port of `GameInfo.swift` / `GameInfo.kt`. Keep titles /
  descriptions / variant lists byte-identical with the iOS and Android sources so the App
  Store / Play Store / Microsoft Store listings stay coherent. Also exposes
  `ALL_FILENAMES` — the list of files the "Spiele aktualisieren" worker downloads.
- `src/index_html.rs` — pure function that renders the game-list "menu" page. Same colour
  palette as iOS / Android (`#263238` background, `#37474F` cards, `#FFD700` accent,
  five-color cycling buttons). The kangaroo logo in the top-right is wrapped in a button
  that fires the `update-games` IPC; while the worker thread runs, the logo gets a spinner
  CSS animation and a bottom toast announces "Spiele werden aktualisiert…", then "X
  Spiele aktualisiert" / "Update fehlgeschlagen: …" once `window.parados_update_done`
  fires.
- `BACK_BUTTON_JS` (in `main.rs`) — injected at document start by wry into every page
  *except* the menu (`/` and `/index.html`). Renders a fixed-position "← Menu" pill that
  navigates back to `parados://localhost/`. Mirrors the auto-hiding back FAB on iOS /
  Android — kept always-visible on desktop because mouse-driven users don't need the
  hide/reveal dance.

The HTML game files live in `assets/games/` and are committed verbatim from
`parados_ios/Parados/Resources/Games/`. Don't modify them in this repo — sync from iOS
when the games change so the three ports stay in lockstep.

## Mac App Store private-API status

`tao` 0.30.x does **not** import `_CGSSetWindowBackgroundBlurRadius` (the symbol that
forces `eframe`/`egui` apps to vendor a `winit-patched/` fork in this workspace). Verified
on the local release build:

```sh
nm -u target/release/parados | grep _CGS    # → only _CGShieldingWindowLevel (public)
```

So we don't need a `[patch.crates-io]` block here. The release workflow re-runs the same
`nm` check inside the `macos-store` job so a future `tao` upgrade that regresses on this
gets caught before App Review.

If a future `tao` release ever imports a private CoreGraphics symbol, follow the same
recipe used in `swissdamed2sqlite` / `eudamed2firstbase` / `rust2xml`: vendor a
`tao-patched/` fork next to this `Cargo.toml`, no-op the offending function body, and add
`[patch.crates-io] tao = { path = "tao-patched" }`.

## Build / run

```sh
cargo build --release           # produces target/release/parados (~4.5 MB stripped)
target/release/parados          # opens the menu window
target/release/parados --url parados://localhost/games/capovolto.html   # deep-link
target/release/parados --screenshot                                    # auto-dismiss rules modals
```

Linux dev deps (CI installs the same set in `release.yml`):

```sh
sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev libxkbcommon-dev \
                 libgtk-3-dev libayatana-appindicator3-dev
```

`cargo run --release --example make_ico` regenerates `assets/icon.ico` from
`assets/icon.png` (multi-resolution PNG-encoded ICO container, sizes 16/24/32/48/64/128/256).
Re-run this whenever the kangaroo source PNG changes; `build.rs` then embeds the .ico into
`parados.exe` via `winresource` on the Windows target.

## Releasing

`.github/workflows/release.yml` triggers on tags matching `vX.Y.Z` (or `vX.Y.Z-rc.N` for
pre-releases). Same workflow shape as `rust2xml`:

- **`build` matrix** — produces tarballs/zips for x86_64 Linux + x86_64 macOS + arm64
  macOS + x86_64 Windows. Each archive bundles `parados`, `README.md`, `LICENSE`. Linux
  archives also include `parados.desktop`, `icon.png` and `install-linux.sh`. macOS
  archives ship a `Parados.app` bundle generated on the runner via `iconutil` from
  `assets/icon.png`.
- **`macos-store` job** (gated on `vars.MACOS_STORE_ENABLED == 'true'`) — builds a
  universal `Parados.app` (lipo'd from x86_64 + arm64), signs it with the Developer ID
  Application identity for a notarized DMG, then re-signs with the Apple Distribution
  identity, runs `productbuild` for a `.pkg`, syncs App Store Connect listing metadata via
  `.github/scripts/appstore_metadata.py`, and uploads via `iTMSTransporter` / `altool`.
- **`windows-msix` job** (gated on `vars.MSSTORE_ENABLED == 'true'`) — packs the GUI +
  `windows/AppxManifest.xml` + `windows/assets/*.png` (5 store logos generated from
  `assets/icon.png` via `sips`) into an MSIX with `makeappx`, optionally co-signs with
  `signtool`, then uploads + commits a Microsoft Store submission via the devcenter REST
  API. The full listing (description / keywords / privacy URL / "what's new") is generated
  inline in PowerShell from the same source-of-truth strings used by the App Store and the
  iOS / Android stores.
- **`publish` job** — collects every artefact and attaches them to a GitHub Release with
  auto-generated notes.

Both store jobs are off by default. Flip on per-repo once the App ID / Microsoft reservation
exist:

```sh
gh variable set MACOS_STORE_ENABLED -R zdavatz/parados_rust -b true
gh variable set MSSTORE_ENABLED     -R zdavatz/parados_rust -b true
gh variable set MSSTORE_APP_ID      -R zdavatz/parados_rust -b "<store app id>"
```

Required secrets — same set as `rust2xml`, can be loaded straight from there:

```
APPLE_TEAM_ID, APPLE_API_KEY_P8, APPLE_API_KEY_ID, APPLE_API_ISSUER_ID,
MACOS_APP_ID,                                         # App Store Connect numeric app id
MACOS_CERTIFICATE (+_PASSWORD),
MACOS_INSTALLER_CERTIFICATE (+_PASSWORD),
MACOS_DEVELOPER_ID_CERTIFICATE (+_PASSWORD),
MACOS_PROVISIONING_PROFILE,
WINDOWS_CERTIFICATE (+_PASSWORD)                      # optional MSIX co-sign
MSSTORE_TENANT_ID, MSSTORE_CLIENT_ID, MSSTORE_CLIENT_SECRET
```

If the gate variables are unset, the matrix build still produces the four tarballs/zips
and the GitHub Release is unchanged — the store steps simply skip.

## Listing copy — single source of truth

The same description / keyword / URL / copyright strings appear in three places. Update all
three when the wording changes:

- `.github/scripts/appstore_metadata.py` — Mac App Store (REST API).
- `.github/workflows/release.yml`, `windows-msix → submit to Microsoft Store` step —
  Microsoft Store (devcenter REST API).
- iOS / Android `README.md` files — Apple Search keywords + Google Play descriptions.

## Game catalog sync

`src/games.rs` mirrors `GameInfo.swift` / `GameInfo.kt`. Whenever Walter Prossnitz adds a
new game or renames an existing one:

1. Add the HTML file to `assets/games/` (copy from `parados_ios/Parados/Resources/Games/`).
2. Append a `Game { ... }` literal to `GAMES` in `src/games.rs` matching the iOS / Android
   entries character-for-character; also extend `ALL_FILENAMES` so the runtime
   "Spiele aktualisieren" worker downloads the new file.
3. Re-run `cargo build --release` (no codegen step needed — `include_dir!` picks up the
   new file at compile time).

For *content-only* updates (HTML game logic changes — no new files, no metadata changes),
users on already-installed builds can hit the kangaroo top-right and pull fresh HTML from
GitHub at runtime; for new files / metadata changes a fresh signed release is still
needed.

## Mac App Store screenshots

`screenshots/macos/` holds the eight 2560×1600 PNGs uploaded to App Store Connect (one
menu + one per game). `screenshots/macos/capture.sh` regenerates them by launching parados
eight times with `--url <game> --screenshot`, resizing to 1280×800 logical (= 2560×1600
physical on Retina) via System Events, and `screencapture`-ing the window. Re-run after
any UI change in `index_html.rs` or any visible game-HTML change.

## Related projects in this workspace

- `parados_ios` — SwiftUI / WKWebView source of the game catalog.
- `parados_android` — Kotlin / WebView reference port.
- `rust2xml` — sibling Rust GUI app whose release workflow this one is modelled on.
