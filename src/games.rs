//! File manifest for "Spiele aktualisieren".
//!
//! The desktop app no longer renders its own menu from a hardcoded game
//! list — the landing page is now the shared `index.html` (the SAME file
//! the website + iOS + Android ship), served by `main.rs` and refreshed by
//! the update button.  All that remains here is the list of files the
//! update button pulls from `raw.githubusercontent.com/zdavatz/parados/main/`.
//!
//! Keep this in sync with `GameInfo.allFilenames` in the iOS / Android
//! sources and with the files actually shipped in the web repo.
pub const ALL_FILENAMES: &[&str] = &[
    "index.html",
    "kangaroo.html", "kangaroo_en.html", "kangaroo_jp.html",
    "kangaroo_cn.html", "kangaroo_ua.html",
    "capovolto.html",
    "divided_loyalties.html", "divided_loyalties_en.html",
    "divided_loyalties_starting_positions.csv",
    "democracy.html", "democracy_en.html", "democracy_remote.html",
    "frankenstein.html",
    "rainbow_blackjack.html", "rainbow_blackjack_en.html",
    "rainbow_blackjack_remote.html",
    "makalaina.html", "makalaina_remote.html",
    "startpositionen.html",
];
