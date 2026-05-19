"""Download upstream KJV and Strong's sources into the local cache."""

from __future__ import annotations

import argparse
import urllib.request
from pathlib import Path

from books import BOOKS
from db import REPO_ROOT

KJV_RAW = "https://raw.githubusercontent.com/kaiserlik/kjv/main"
STRONGS_RAW = "https://raw.githubusercontent.com/openscriptures/strongs/master"


def _kjv_json_usable(path: Path, expected_abbrev: str) -> bool:
    """True when the cached book file yields at least one English verse."""
    try:
        from parse_kjv import parse_book_json

        verses = parse_book_json(path)
        return len(verses) > 0
    except (OSError, ValueError):
        return False


def _download(url: str, dest: Path, *, expected_abbrev: str | None = None) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.exists() and dest.stat().st_size > 0:
        if expected_abbrev is None or _kjv_json_usable(dest, expected_abbrev):
            print(f"  cached {dest.name}")
            return
        print(f"  stale {dest.name}, re-fetching")
        dest.unlink()
    print(f"  fetch {url}")
    urllib.request.urlretrieve(url, dest)
    if expected_abbrev is not None and not _kjv_json_usable(dest, expected_abbrev):
        dest.unlink(missing_ok=True)
        raise RuntimeError(f"downloaded KJV file is unusable: {dest.name}")


def fetch_kjv(cache_dir: Path, *, books: list[str] | None = None) -> Path:
    kjv_dir = cache_dir / "kjv"
    abbrevs = books or [b.abbrev for b in BOOKS]
    for abbrev in abbrevs:
        _download(
            f"{KJV_RAW}/{abbrev}.json",
            kjv_dir / f"{abbrev}.json",
            expected_abbrev=abbrev,
        )
    return kjv_dir


def fetch_strongs(cache_dir: Path) -> Path:
    strongs_dir = cache_dir / "strongs"
    _download(
        f"{STRONGS_RAW}/greek/strongs-greek-dictionary.js",
        strongs_dir / "greek" / "strongs-greek-dictionary.js",
    )
    _download(
        f"{STRONGS_RAW}/hebrew/strongs-hebrew-dictionary.js",
        strongs_dir / "hebrew" / "strongs-hebrew-dictionary.js",
    )
    return strongs_dir


def main() -> None:
    parser = argparse.ArgumentParser(description="Fetch fontes upstream sources")
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=REPO_ROOT / ".cache" / "fontes" / "sources",
    )
    parser.add_argument(
        "--books",
        nargs="*",
        help="KJV book abbrevs to download (default: all 66)",
    )
    parser.add_argument(
        "--strongs-only",
        action="store_true",
        help="Only download Strong's dictionaries",
    )
    parser.add_argument(
        "--kjv-only",
        action="store_true",
        help="Only download KJV JSON",
    )
    args = parser.parse_args()

    if not args.kjv_only:
        print("Strong's…")
        fetch_strongs(args.cache_dir)
    if not args.strongs_only:
        print("KJV…")
        fetch_kjv(args.cache_dir, books=args.books)

    print(f"cache ready at {args.cache_dir}")


if __name__ == "__main__":
    main()
