#!/usr/bin/env python3
"""Fetch all sources and build a full KJV + Strong's content bundle."""

from __future__ import annotations

import sys
from pathlib import Path

_ROOT = Path(__file__).resolve().parent
if str(_ROOT) not in sys.path:
    sys.path.insert(0, str(_ROOT))

from build_content import REPO_ROOT, build_content_db, write_manifest  # noqa: E402
from db import connect, init_user_db  # noqa: E402
from fetch_sources import fetch_kjv, fetch_strongs  # noqa: E402

BUNDLES = REPO_ROOT / "data" / "bundles"
CACHE = REPO_ROOT / ".cache" / "fontes" / "sources"
VERSION = "kjv-strongs-1.0.0"


def main() -> None:
    print("Fetching all KJV + Strong's sources…")
    fetch_strongs(CACHE)
    fetch_kjv(CACHE)

    content_db = BUNDLES / "content.sqlite"
    manifest = BUNDLES / "manifest.json"
    bundle_zip = BUNDLES / f"fontes-core-{VERSION}.zip"

    print("Building full content.sqlite…")
    build_content_db(
        content_db,
        CACHE / "kjv",
        CACHE / "strongs",
        bundle_version=VERSION,
        scope="full",
    )
    write_manifest(
        manifest,
        bundle_version=VERSION,
        content_db=content_db,
        scope="full",
    )

    import zipfile

    print(f"Writing {bundle_zip}…")
    BUNDLES.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(bundle_zip, "w", zipfile.ZIP_DEFLATED) as zf:
        zf.write(content_db, "content.sqlite")
        zf.write(manifest, "manifest.json")

    # Dev user db beside fixtures for convenience.
    fixtures = REPO_ROOT / "data" / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)
    import shutil

    shutil.copy2(content_db, fixtures / "content.sqlite")
    shutil.copy2(manifest, fixtures / "manifest.json")
    user_db = fixtures / "user.sqlite"
    if not user_db.exists():
        conn = connect(user_db)
        init_user_db(conn)
        conn.close()

    size_mb = content_db.stat().st_size / (1024 * 1024)
    print(f"Done.\n  {content_db} ({size_mb:.1f} MB)\n  {bundle_zip}")


if __name__ == "__main__":
    main()
