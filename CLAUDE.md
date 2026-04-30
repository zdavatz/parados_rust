# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project overview

Rust desktop port of the Parados Think Ahead board games. Same seven HTML games shipped by
`parados_ios` (SwiftUI / WKWebView) and `parados_android` (RecyclerView / WebView), now
running cross-platform on Linux / macOS / Windows via `tao` (window) + `wry` (webview).
GPL-3.0.

Bundle ID: `com.ywesee.parados` · Microsoft Store reservation: `yweseeGmbH.parados` · same
bundle ID and asset set as the iOS / Android editions, deliberately, so all three stores
list the same product family.

## Architecture

Single binary (`parados`) — no library, no helper crates. Runtime structure:

- `src/main.rs` — opens a `tao` window with the kangaroo PNG decoded into a native icon,
  mounts a `wry` webview pointing at `parados://localhost/`. The custom-protocol handler
  serves the menu page (rendered by `index_html::render`) and every `assets/games/*.html`
  file (embedded via `include_dir!`). An IPC handler accepts `open-external:<url>` messages
  from the menu page and hands them to the user's default browser via the `open` crate —
  this is how the three remote-multiplayer variants reach `https://game.ywesee.com/parados/`.
- `src/games.rs` — direct port of `GameInfo.swift` / `GameInfo.kt`. Keep titles /
  descriptions / variant lists byte-identical with the iOS and Android sources so the App
  Store / Play Store / Microsoft Store listings stay coherent.
- `src/index_html.rs` — pure function that renders the game-list "menu" page. Same colour
  palette as iOS / Android (`#263238` background, `#37474F` cards, `#FFD700` accent,
  five-color cycling buttons). Kangaroo logo in the top-right via
  `parados://localhost/assets/kangy.jpg`.
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
cargo build --release           # produces target/release/parados (~2.5 MB stripped)
target/release/parados          # opens the menu window
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
   entries character-for-character.
3. Re-run `cargo build --release` (no codegen step needed — `include_dir!` picks up the
   new file at compile time).

There's no "Spiele aktualisieren" GitHub-update path on desktop — users get new games via
the next signed release, the same way they get app updates from any store. Keeping the
runtime simple here is intentional; the iOS / Android live-update path exists because the
mobile stores have multi-week review queues, and that pressure doesn't apply to a desktop
binary the user can refresh in 30 seconds.

## Related projects in this workspace

- `parados_ios` — SwiftUI / WKWebView source of the game catalog.
- `parados_android` — Kotlin / WebView reference port.
- `rust2xml` — sibling Rust GUI app whose release workflow this one is modelled on.
