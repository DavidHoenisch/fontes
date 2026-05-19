#!/usr/bin/env python3
"""Fetch sources and build dev fixtures (John 1–3 + full Strong's)."""

from __future__ import annotations

import sys
from pathlib import Path

_ROOT = Path(__file__).resolve().parent
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from build_content import build_content_db, write_manifest  # noqa: E402
from db import REPO_ROOT, connect, init_user_db  # noqa: E402
from fetch_sources import fetch_kjv, fetch_strongs  # noqa: E402

FIXTURES = REPO_ROOT / "data" / "fixtures"
CACHE = REPO_ROOT / ".cache" / "fontes" / "sources"


def main() -> None:
    print("Fetching sources…")
    fetch_strongs(CACHE)
    fetch_kjv(CACHE, books=["Jhn"])

    content_db = FIXTURES / "content.sqlite"
    user_db = FIXTURES / "user.sqlite"
    manifest = FIXTURES / "manifest.json"

    print("Building content.sqlite (John 1–3)…")
    build_content_db(
        content_db,
        CACHE / "kjv",
        CACHE / "strongs",
        books=["Jhn"],
        chapters={1, 2, 3},
        bundle_version="dev-jhn-1-3",
        scope="books:Jhn;chapters:1,2,3",
    )
    write_manifest(
        manifest,
        bundle_version="dev-jhn-1-3",
        content_db=content_db,
        scope="books:Jhn;chapters:1,2,3",
    )

    if user_db.exists():
        user_db.unlink()
    print("Initializing user.sqlite…")
    conn = connect(user_db)
    try:
        init_user_db(conn)
        conn.commit()
    finally:
        conn.close()

    print(f"Done.\n  {content_db}\n  {user_db}\n  {manifest}")


if __name__ == "__main__":
    main()
