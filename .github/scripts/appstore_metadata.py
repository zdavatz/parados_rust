#!/usr/bin/env python3
"""
Push the Parados App Store Connect listing to api.appstoreconnect.apple.com.

Mirrors what the rust2xml workflow does: signs a JWT with the Apple
API .p8 key, finds the EDITABLE iOS / macOS app store version for
this build, then PATCHes:

  * appInfoLocalizations  — name, subtitle, privacy URL
  * appStoreVersionLocalizations — description, keywords, marketing
                                   / support URLs, "what's new"
  * appStoreVersions      — copyright, release type

Description / keywords / URLs come from the same source-of-truth
strings the iOS, Android and Windows store listings use, so all
three stores stay in sync.

Idempotent — re-running on the same App Store version overwrites
the existing fields with identical values.

Failure is non-fatal: the calling workflow step suppresses errors
so an upload that already happened isn't blocked by a metadata
hiccup.
"""

from __future__ import annotations

import argparse
import base64
import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

API_BASE = "https://api.appstoreconnect.apple.com/v1"

# ---------------------------------------------------------------------
# Source-of-truth listing copy.  Keep aligned with:
#   - parados_ios/README.md       (App Store)
#   - parados_android/README.md   (Google Play)
#   - .github/workflows/release.yml  (Microsoft Store)
# ---------------------------------------------------------------------

DESCRIPTION_EN = (
    "Parados is a collection of seven original board games — playable offline, "
    "fully embedded, no internet connection required.\n\n"
    "DUK — The Impatient Kangaroo (1 player, puzzle, DE/EN/JP/CN/UA)\n"
    "Capovolto (2 players, strategy)\n"
    "Divided Loyalties (2 players, strategy)\n"
    "Democracy in Space (2+ players, strategy; remote multiplayer optional)\n"
    "Frankenstein — Where's that green elbow? (1–4 players, memory)\n"
    "Rainbow Blackjack (2 players, strategy; remote multiplayer optional)\n"
    "MAKA LAINA (2 players, strategy; remote multiplayer optional)\n\n"
    "Three games offer optional cross-device PeerJS / WebRTC multiplayer; "
    "those open in your default browser. All other games run entirely offline.\n\n"
    "Open Source (GPL v3) — by Walter Prossnitz / Think Ahead Games / ywesee GmbH."
)
DESCRIPTION_DE = (
    "Parados ist eine Sammlung von sieben originalen Brettspielen — offline "
    "spielbar, vollständig eingebettet, kein Internet nötig.\n\n"
    "DUK — Das ungeduldige Känguru (1 Spieler, Puzzle, DE/EN/JP/CN/UA)\n"
    "Capovolto (2 Spieler, Strategie)\n"
    "Divided Loyalties (2 Spieler, Strategie)\n"
    "Democracy in Space (2+ Spieler, Strategie; optional Remote-Multiplayer)\n"
    "Frankenstein — Wo ist der grüne Ellbogen? (1–4 Spieler, Memory)\n"
    "Rainbow Blackjack (2 Spieler, Strategie; optional Remote-Multiplayer)\n"
    "MAKA LAINA (2 Spieler, Strategie; optional Remote-Multiplayer)\n\n"
    "Drei Spiele bieten optional PeerJS / WebRTC Mehrspieler-Modus; "
    "diese öffnen im Standard-Browser. Alle anderen Spiele laufen komplett offline.\n\n"
    "Open Source (GPL v3) — von Walter Prossnitz / Think Ahead Games / ywesee GmbH."
)
KEYWORDS_EN = "Parados,Think Ahead,Board Games,Puzzle,Strategy,Memory,Offline,Kangaroo,Othello"
KEYWORDS_DE = "Parados,Think Ahead,Brettspiele,Puzzle,Strategie,Memory,Offline,Känguru,Othello"
SUBTITLE_EN = "Seven Think Ahead Board Games"
SUBTITLE_DE = "Sieben Think-Ahead-Brettspiele"
PROMOTIONAL_EN = "Seven original Think Ahead board games — embedded, offline, no ads."
PROMOTIONAL_DE = "Sieben originale Think-Ahead-Brettspiele — eingebettet, offline, ohne Werbung."
WHATS_NEW_EN = "Initial release of the Parados desktop port (macOS Mac App Store)."
WHATS_NEW_DE = "Erstveröffentlichung der Parados-Desktop-Version (macOS Mac App Store)."
SUPPORT_URL = "https://ywesee.com/Parados/Parados"
MARKETING_URL = "https://game.ywesee.com/parados/"
PRIVACY_URL = "https://ywesee.com/Parados/Parados"
COPYRIGHT = "Think Ahead Games / ywesee GmbH"


# ---------------------------------------------------------------------
# JWT signing.  No PyJWT dependency on GitHub-hosted runners — sign
# ES256 manually with `cryptography` if available, otherwise shell
# out to `openssl`.
# ---------------------------------------------------------------------


def jwt_token(key_id: str, issuer_id: str, key_path: Path) -> str:
    header = {"alg": "ES256", "kid": key_id, "typ": "JWT"}
    payload = {
        "iss": issuer_id,
        "iat": int(time.time()),
        "exp": int(time.time()) + 600,
        "aud": "appstoreconnect-v1",
    }

    def b64(data: bytes) -> str:
        return base64.urlsafe_b64encode(data).rstrip(b"=").decode()

    msg = (
        b64(json.dumps(header,  separators=(",", ":")).encode())
        + "."
        + b64(json.dumps(payload, separators=(",", ":")).encode())
    )

    try:
        from cryptography.hazmat.primitives import hashes, serialization
        from cryptography.hazmat.primitives.asymmetric import ec
        from cryptography.hazmat.primitives.asymmetric.utils import (
            decode_dss_signature,
        )

        pem = key_path.read_bytes()
        private_key = serialization.load_pem_private_key(pem, password=None)
        sig_der = private_key.sign(msg.encode(), ec.ECDSA(hashes.SHA256()))
        r, s = decode_dss_signature(sig_der)
        sig = r.to_bytes(32, "big") + s.to_bytes(32, "big")
        return msg + "." + b64(sig)
    except Exception as e:
        print(f"warning: cryptography unavailable ({e}); falling back to openssl", file=sys.stderr)
        import subprocess

        sig_der = subprocess.check_output(
            ["openssl", "dgst", "-sha256", "-sign", str(key_path)],
            input=msg.encode(),
        )
        # openssl emits DER — strip to raw r||s 64-byte form
        # Manual ASN.1 parse: 0x30 len 0x02 lr r 0x02 ls s
        i = 2
        if sig_der[i] != 0x02:
            raise RuntimeError("unexpected DER signature shape")
        lr = sig_der[i + 1]
        r = sig_der[i + 2 : i + 2 + lr].lstrip(b"\x00")
        i += 2 + lr
        if sig_der[i] != 0x02:
            raise RuntimeError("unexpected DER signature shape")
        ls = sig_der[i + 1]
        s = sig_der[i + 2 : i + 2 + ls].lstrip(b"\x00")
        sig = r.rjust(32, b"\x00") + s.rjust(32, b"\x00")
        return msg + "." + b64(sig)


# ---------------------------------------------------------------------
# Tiny JSON-API helper.
# ---------------------------------------------------------------------


def api(token: str, method: str, path: str, body: dict | None = None) -> dict:
    url = path if path.startswith("http") else f"{API_BASE}{path}"
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Authorization", f"Bearer {token}")
    req.add_header("Content-Type", "application/json")
    try:
        with urllib.request.urlopen(req, timeout=60) as resp:
            raw = resp.read()
            return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        body = e.read().decode(errors="replace")
        raise RuntimeError(f"{method} {path} → {e.code}\n{body}") from None


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument("--app-id",    required=True, help="App Store Connect numeric App ID")
    p.add_argument("--version",   required=True, help="Version string, e.g. 1.0.0")
    p.add_argument("--key-id",    required=True)
    p.add_argument("--issuer-id", required=True)
    p.add_argument("--key-file",  required=True, type=Path)
    args = p.parse_args()

    if not args.key_file.is_file():
        print(f"error: key file not found: {args.key_file}", file=sys.stderr)
        return 1

    token = jwt_token(args.key_id, args.issuer_id, args.key_file)

    # 1. Find the editable macOS App Store version for this build.
    versions = api(token, "GET", f"/apps/{args.app_id}/appStoreVersions?filter[platform]=MAC_OS&limit=10")
    target_version = None
    for v in versions.get("data", []):
        attrs = v.get("attributes", {})
        if attrs.get("versionString") == args.version and attrs.get("appStoreState") in {
            "PREPARE_FOR_SUBMISSION", "DEVELOPER_REJECTED",
            "REJECTED", "METADATA_REJECTED", "INVALID_BINARY",
            "WAITING_FOR_REVIEW",
        }:
            target_version = v
            break
    if target_version is None:
        print(
            f"warning: no editable macOS version {args.version} found — "
            "skipping metadata sync (this is normal if the build hasn't "
            "uploaded yet)",
            file=sys.stderr,
        )
        return 0
    version_id = target_version["id"]
    print(f"editing macOS appStoreVersion {version_id} ({args.version})", file=sys.stderr)

    # 2. Update the appStoreVersion attributes (copyright, release type).
    api(token, "PATCH", f"/appStoreVersions/{version_id}", body={
        "data": {
            "type": "appStoreVersions",
            "id": version_id,
            "attributes": {
                "copyright": COPYRIGHT,
                "releaseType": "MANUAL",
            },
        }
    })

    # 3. Walk the version localizations, patching each language we know.
    locs = api(token, "GET", f"/appStoreVersions/{version_id}/appStoreVersionLocalizations?limit=200")
    by_locale = {l["attributes"]["locale"]: l["id"] for l in locs.get("data", [])}

    locale_payload = {
        "en-US": dict(
            description=DESCRIPTION_EN,
            keywords=KEYWORDS_EN,
            marketingUrl=MARKETING_URL,
            supportUrl=SUPPORT_URL,
            promotionalText=PROMOTIONAL_EN,
            whatsNew=WHATS_NEW_EN,
        ),
        "de-DE": dict(
            description=DESCRIPTION_DE,
            keywords=KEYWORDS_DE,
            marketingUrl=MARKETING_URL,
            supportUrl=SUPPORT_URL,
            promotionalText=PROMOTIONAL_DE,
            whatsNew=WHATS_NEW_DE,
        ),
    }
    for locale, attrs in locale_payload.items():
        loc_id = by_locale.get(locale)
        if not loc_id:
            print(f"warning: {locale} versionLocalization missing — skipped", file=sys.stderr)
            continue
        api(token, "PATCH",
            f"/appStoreVersionLocalizations/{loc_id}",
            body={"data": {
                "type": "appStoreVersionLocalizations",
                "id": loc_id,
                "attributes": attrs,
            }},
        )
        print(f"  + version localization {locale} updated", file=sys.stderr)

    # 4. App-level metadata (name / subtitle / privacy URL) lives on
    #    the appInfo, not the version.  Fetch the EDITABLE one and
    #    patch its localizations.
    infos = api(token, "GET", f"/apps/{args.app_id}/appInfos?limit=10")
    editable_info = None
    for ai in infos.get("data", []):
        state = ai.get("attributes", {}).get("appStoreState")
        if state in {
            "PREPARE_FOR_SUBMISSION", "DEVELOPER_REJECTED",
            "REJECTED", "METADATA_REJECTED",
            "WAITING_FOR_REVIEW",
        }:
            editable_info = ai
            break
    if editable_info is None:
        print("warning: no editable appInfo — skipping name/subtitle update", file=sys.stderr)
    else:
        info_id = editable_info["id"]
        info_locs = api(token, "GET", f"/appInfos/{info_id}/appInfoLocalizations?limit=200")
        info_by_locale = {l["attributes"]["locale"]: l["id"] for l in info_locs.get("data", [])}
        info_payload = {
            "en-US": dict(name="Parados", subtitle=SUBTITLE_EN, privacyPolicyUrl=PRIVACY_URL),
            "de-DE": dict(name="Parados", subtitle=SUBTITLE_DE, privacyPolicyUrl=PRIVACY_URL),
        }
        for locale, attrs in info_payload.items():
            loc_id = info_by_locale.get(locale)
            if not loc_id:
                print(f"warning: {locale} appInfoLocalization missing — skipped", file=sys.stderr)
                continue
            api(token, "PATCH",
                f"/appInfoLocalizations/{loc_id}",
                body={"data": {
                    "type": "appInfoLocalizations",
                    "id": loc_id,
                    "attributes": attrs,
                }},
            )
            print(f"  + app-info localization {locale} updated", file=sys.stderr)

    print("App Store Connect metadata sync complete.", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
