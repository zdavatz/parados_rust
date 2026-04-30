# Parados — Desktop

Rust desktop port of the [Parados](https://game.ywesee.com/parados/) Think Ahead board games.
Same seven games as the [iOS](https://github.com/zdavatz/parados_ios) and
[Android](https://github.com/zdavatz/parados_android) editions, now playable offline on Linux,
macOS and Windows.

## Games

- **DUK — The Impatient Kangaroo** (1 player, puzzle, DE/EN/JP/CN/UA)
- **Capovolto** (2 players, strategy)
- **Divided Loyalties** (2 players, strategy)
- **Democracy in Space** (2+ players, strategy — optional remote multiplayer)
- **Frankenstein — Where's that green elbow?** (1–4 players, memory)
- **Rainbow Blackjack** (2 players, strategy — optional remote multiplayer)
- **MAKA LAINA** (2 players, strategy — optional remote multiplayer)

Three games offer optional PeerJS / WebRTC multiplayer; those variants open in your default
browser at `https://game.ywesee.com/parados/`. All other games run entirely offline inside
the embedded webview (WKWebView on macOS, WebView2 on Windows, WebKitGTK on Linux).

The window title bar shows the running version (e.g. `Parados 1.0.2`) on every platform,
so users can see at a glance which build they're on without an About dialog.

## Refreshing games at runtime

Click the kangaroo logo in the top-right of the menu page to download fresh game HTML from
`raw.githubusercontent.com/zdavatz/parados/main/` into a per-app data directory:

- macOS: `~/Library/Application Support/Parados/games/`
- Windows: `%APPDATA%\Parados\games\`
- Linux: `~/.local/share/parados/games/`

The custom `parados://` protocol handler reads from this overlay first, falling back to
the bundle baked into the binary at compile time. Refreshes survive across launches. Same
UX as the iOS Menu / Android toolbar entry "Spiele aktualisieren".

## Build

```sh
cargo build --release
target/release/parados
```

Linux build deps (Ubuntu/Debian):

```sh
sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev libxkbcommon-dev \
                 libgtk-3-dev libayatana-appindicator3-dev
```

## Releases

Tagging `vX.Y.Z` triggers `.github/workflows/release.yml` which builds binaries for:

- `x86_64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

…and (when the store gates are enabled) signs a Mac App Store `.pkg` and uploads it to App
Store Connect via `iTMSTransporter` / `altool`, plus a Microsoft Store `.msix` uploaded via
the Partner Center REST API. Listing copy (description / keywords / privacy URL / etc.) is
shared with the iOS / Android stores via `.github/scripts/appstore_metadata.py` and the
inline `windows-msix` step.

Release flow:

```sh
# bump version in Cargo.toml first, then:
git commit -am "Release vX.Y.Z"
git push
git tag vX.Y.Z
git push origin vX.Y.Z
```

## License

GPL-3.0-only — see [LICENSE](LICENSE). Games © Walter Prossnitz / Think Ahead Games.
