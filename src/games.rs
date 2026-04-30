//! Game catalog — direct port of `Parados/Models/GameInfo.swift`
//! (iOS) and `app/src/main/java/com/ywesee/parados/GameInfo.kt`
//! (Android).  Keep titles / descriptions / variants byte-identical
//! across the three ports so the App Store / Play Store / Microsoft
//! Store listings stay in sync.

pub struct Variant {
    pub filename: &'static str,
    pub label: &'static str,
    /// Set on remote-multiplayer variants: opens in the default
    /// browser instead of loading inside the embedded webview, because
    /// PeerJS / WebRTC require an `https://` origin and a `parados://`
    /// custom-scheme page is treated as insecure by every browser
    /// engine.  Same behaviour as iOS / Android.
    pub url: Option<&'static str>,
}

pub struct Game {
    pub filename: &'static str,
    pub title: &'static str,
    pub players: &'static str,
    pub description: &'static str,
    pub variants: &'static [Variant],
}

/// Every HTML / CSV file the app ships, used by "Spiele aktualisieren"
/// to know which files to refresh from GitHub.  Mirrors
/// `GameInfo.allFilenames` in the iOS / Android sources.
pub const ALL_FILENAMES: &[&str] = &[
    "index.html",
    "kangaroo.html", "kangaroo_en.html", "kangaroo_jp.html",
    "kangaroo_cn.html", "kangaroo_ua.html",
    "capovolto.html",
    "divided_loyalties.html",
    "democracy.html", "democracy_remote.html",
    "frankenstein.html",
    "rainbow_blackjack.html", "rainbow_blackjack_en.html",
    "rainbow_blackjack_remote.html",
    "makalaina.html", "makalaina_remote.html",
    "makalaina_starting_positions.csv",
];

/// Used by the index-page renderer.  Keep the order identical to
/// `GameInfo.allGames()` in the iOS / Android sources.
pub const GAMES: &[Game] = &[
    Game {
        filename: "kangaroo.html",
        title: "DUK \u{2014} The Impatient Kangaroo",
        players: "1 Player \u{00b7} Puzzle",
        description: "The plan: to create a 21st century successor to the super-hit solo puzzle game Rushhour. Many players think that hopping through the outback collecting goodies is more fun than trying to shove your way through traffic? Another advantage \u{2014} thanks to the program, there's all kinds of different ways to play:-)",
        variants: &[
            Variant { filename: "kangaroo.html",    label: "DE", url: None },
            Variant { filename: "kangaroo_en.html", label: "EN", url: None },
            Variant { filename: "kangaroo_jp.html", label: "JP", url: None },
            Variant { filename: "kangaroo_cn.html", label: "CN", url: None },
            Variant { filename: "kangaroo_ua.html", label: "UA", url: None },
        ],
    },
    Game {
        filename: "capovolto.html",
        title: "Capovolto",
        players: "2 Players \u{00b7} Strategy",
        description: "The classic game of Othello \u{2014} on steroids! Add in area control on a random board, numbered discs and a flipping mechanism that is light years ahead of the original, inviting all kinds of devious strategies, and designed to make your brain go all sorts of places it hasn't been before:-)",
        variants: &[],
    },
    Game {
        filename: "divided_loyalties.html",
        title: "Divided Loyalties",
        players: "2 Players \u{00b7} Strategy",
        description: "Many turns are offensive AND defensive and each one may have long term consequences! It's connect 4, but with 6 colours. Your colour is always loyal to you, your opponent's never is... and the other 4? Sometimes they are, and sometimes they aren't. Tiles can even be loyal in one direction, AND disloyal in another! Not for the faint of heart....",
        variants: &[],
    },
    Game {
        filename: "democracy.html",
        title: "Democracy in Space",
        players: "2+ Players \u{00b7} Strategy",
        description: "Based on the concept of the US Electoral College (parliamentary systems also have it). Area majority culled to its essence. A gentle opening suddenly transforms into a nail biting race to the finish! The tie breaker condition needs to be kept in mind, but you won't know for a while if you'll need it this time....",
        variants: &[
            Variant { filename: "democracy.html", label: "Play", url: None },
            Variant {
                filename: "democracy_remote.html",
                label: "Remote",
                url: Some("https://game.ywesee.com/parados/democracy_remote.html"),
            },
        ],
    },
    Game {
        filename: "frankenstein.html",
        title: "Frankenstein \u{2014} Where's that green elbow?",
        players: "1\u{2013}4 Players \u{00b7} Memory",
        description: "This is even shorter and sweeter than Rainbow. For 1\u{2013}4 players, it's a \"frankly memorable\" game (you'll get the pun when you play it). Like most of its colleagues here at Think Ahead, it is so much easier to play online. Age recommendation \u{2014} 7 years and up. Don't be surprised if the youngest player wins:-)).",
        variants: &[],
    },
    Game {
        filename: "rainbow_blackjack.html",
        title: "Rainbow Blackjack",
        players: "2 Players \u{00b7} Strategy",
        description: "Colorful 21! Two players build 6 colored towers, trying to get as close to 21 as possible \u{2014} like Blackjack, but with colored stones. This game is easier to play than to describe:-) Arrange your stones in a grid, pick rows wisely, and announce just enough to keep your opponent guessing. Gray jokers add a devious twist...",
        variants: &[
            Variant { filename: "rainbow_blackjack.html",    label: "Deutsch", url: None },
            Variant { filename: "rainbow_blackjack_en.html", label: "English", url: None },
            Variant {
                filename: "rainbow_blackjack_remote.html",
                label: "Remote",
                url: Some("https://game.ywesee.com/parados/rainbow_blackjack_remote.html"),
            },
        ],
    },
    Game {
        filename: "makalaina.html",
        title: "MAKA LAINA",
        players: "2 Players \u{00b7} Strategy",
        description: "It's the first turn and the battle is on! No time to get warmed up in MakaLaina:-) You need to be planning from the get go, evolving your long term strategy \u{2014} but staying flexible. The constant influx of new discs means that even a small shift can have consequences...",
        variants: &[
            Variant { filename: "makalaina.html", label: "Play", url: None },
            Variant {
                filename: "makalaina_remote.html",
                label: "Remote",
                url: Some("https://game.ywesee.com/parados/makalaina_remote.html"),
            },
        ],
    },
];
