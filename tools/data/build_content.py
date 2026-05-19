"""Build content.sqlite from cached upstream sources."""

from __future__ import annotations

import argparse
import hashlib
import json
import sqlite3
from datetime import UTC, datetime
from pathlib import Path

from books import BOOKS
from db import REPO_ROOT, connect, init_content_db
from parse_kjv import parse_book_json
from parse_strongs import StrongEntry, parse_strongs_dictionary

KJV_TRANSLATION_ID = 1


def _insert_books(conn: sqlite3.Connection) -> None:
    conn.executemany(
        """
        INSERT INTO book (id, osis, abbrev, name, testament, sort_order)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        [(b.id, b.osis, b.abbrev, b.name, b.testament, b.id) for b in BOOKS],
    )


def _insert_translation(conn: sqlite3.Connection) -> None:
    conn.execute(
        """
        INSERT INTO translation (id, code, name, language, license)
        VALUES (1, 'kjv', 'King James Version', 'en',
                'Public Domain (text); Strong''s tags from kaiserlik/kjv')
        """
    )


def _verse_id(book_id: int, chapter: int, verse: int) -> int:
    return book_id * 1_000_000 + chapter * 1_000 + verse


def ingest_kjv(
    conn: sqlite3.Connection,
    kjv_dir: Path,
    *,
    books: list[str] | None,
    chapters: set[int] | None,
) -> tuple[int, int, int]:
    verse_rows: list[tuple[int, int, int, int]] = []
    text_rows: list[tuple[int, int, str]] = []
    token_rows: list[tuple[int, int, int, str, str | None, int]] = []
    occurrence_rows: list[tuple[str, int, int, int]] = []

    abbrevs = books or [b.abbrev for b in BOOKS]
    for abbrev in abbrevs:
        path = kjv_dir / f"{abbrev}.json"
        if not path.exists():
            raise FileNotFoundError(f"missing KJV book file: {path}")
        for parsed in parse_book_json(path, chapters=chapters):
            vid = _verse_id(parsed.book.id, parsed.chapter, parsed.verse)
            verse_rows.append((vid, parsed.book.id, parsed.chapter, parsed.verse))
            text_rows.append((vid, KJV_TRANSLATION_ID, parsed.plain_text))
            for idx, tok in enumerate(parsed.tokens):
                token_rows.append(
                    (
                        vid,
                        KJV_TRANSLATION_ID,
                        idx,
                        tok.surface,
                        tok.strong_key,
                        tok.flags,
                    )
                )
                if tok.strong_key:
                    occurrence_rows.append(
                        (tok.strong_key, KJV_TRANSLATION_ID, vid, idx)
                    )

    conn.executemany(
        "INSERT INTO verse (id, book_id, chapter, verse) VALUES (?, ?, ?, ?)",
        verse_rows,
    )
    conn.executemany(
        "INSERT INTO verse_text (verse_id, translation_id, text) VALUES (?, ?, ?)",
        text_rows,
    )
    conn.executemany(
        """
        INSERT INTO token
          (verse_id, translation_id, idx, surface, strong_key, flags)
        VALUES (?, ?, ?, ?, ?, ?)
        """,
        token_rows,
    )
    conn.executemany(
        """
        INSERT INTO strong_occurrence
          (strong_key, translation_id, verse_id, token_idx)
        VALUES (?, ?, ?, ?)
        """,
        occurrence_rows,
    )
    return len(verse_rows), len(token_rows), len(occurrence_rows)


def ingest_strongs(conn: sqlite3.Connection, strongs_dir: Path) -> tuple[int, int]:
    greek = parse_strongs_dictionary(
        strongs_dir / "greek" / "strongs-greek-dictionary.js", "greek"
    )
    hebrew = parse_strongs_dictionary(
        strongs_dir / "hebrew" / "strongs-hebrew-dictionary.js", "hebrew"
    )
    all_entries = greek + hebrew

    conn.executemany(
        """
        INSERT INTO strong_entry
          (key, lang, lemma, translit, definition, kjv_gloss, derivation)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        """,
        [
            (
                e.key,
                e.lang,
                e.lemma,
                e.translit,
                e.definition,
                e.kjv_gloss,
                e.derivation,
            )
            for e in all_entries
        ],
    )

    keys = {e.key for e in all_entries}
    xref_rows: list[tuple[str, str]] = []
    for entry in all_entries:
        for to_key in entry.xrefs:
            if to_key != entry.key and to_key in keys:
                xref_rows.append((entry.key, to_key))
    conn.executemany(
        "INSERT OR IGNORE INTO strong_xref (from_key, to_key) VALUES (?, ?)",
        xref_rows,
    )
    return len(all_entries), len(xref_rows)


def _write_bundle_meta(
    conn: sqlite3.Connection,
    *,
    bundle_version: str,
    scope: str,
) -> None:
    now = datetime.now(UTC).isoformat()
    conn.executemany(
        "INSERT INTO bundle_meta (key, value) VALUES (?, ?)",
        [
            ("bundle_version", bundle_version),
            ("built_at", now),
            ("scope", scope),
        ],
    )


def build_content_db(
    output: Path,
    kjv_dir: Path,
    strongs_dir: Path,
    *,
    books: list[str] | None = None,
    chapters: set[int] | None = None,
    bundle_version: str = "dev",
    scope: str = "full",
) -> None:
    if output.exists():
        output.unlink()

    conn = connect(output)
    try:
        init_content_db(conn)
        _insert_translation(conn)
        _insert_books(conn)
        ingest_strongs(conn, strongs_dir)
        ingest_kjv(conn, kjv_dir, books=books, chapters=chapters)
        _write_bundle_meta(conn, bundle_version=bundle_version, scope=scope)
        conn.commit()
        conn.execute("VACUUM")
    finally:
        conn.close()


def write_manifest(
    manifest_path: Path,
    *,
    bundle_version: str,
    content_db: Path,
    scope: str,
) -> None:
    digest = hashlib.sha256(content_db.read_bytes()).hexdigest()
    manifest = {
        "bundle_id": "fontes-core",
        "version": bundle_version,
        "scope": scope,
        "content_db_sha256": digest,
        "content_db_bytes": content_db.stat().st_size,
    }
    manifest_path.write_text(
        json.dumps(manifest, indent=2) + "\n", encoding="utf-8"
    )


def main() -> None:
    parser = argparse.ArgumentParser(description="Build fontes content.sqlite")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        default=REPO_ROOT / "data" / "fixtures" / "content.sqlite",
    )
    parser.add_argument(
        "--cache-dir",
        type=Path,
        default=REPO_ROOT / ".cache" / "fontes" / "sources",
    )
    parser.add_argument(
        "--books",
        nargs="*",
        help="Book abbrevs to include (default: all)",
    )
    parser.add_argument(
        "--chapters",
        type=int,
        nargs="*",
        help="If set, only include these chapter numbers per book",
    )
    parser.add_argument("--bundle-version", default="dev")
    parser.add_argument(
        "--manifest",
        type=Path,
        help="Optional manifest.json output path",
    )
    parser.add_argument(
        "--zip",
        type=Path,
        help="Optional fontes-core bundle zip (content.sqlite + manifest.json)",
    )
    args = parser.parse_args()

    kjv_dir = args.cache_dir / "kjv"
    strongs_dir = args.cache_dir / "strongs"
    chapters = set(args.chapters) if args.chapters else None
    scope = "full"
    if args.books:
        scope = f"books:{','.join(args.books)}"
    if chapters:
        scope += f";chapters:{','.join(map(str, sorted(chapters)))}"

    build_content_db(
        args.output,
        kjv_dir,
        strongs_dir,
        books=args.books,
        chapters=chapters,
        bundle_version=args.bundle_version,
        scope=scope,
    )

    manifest_path = args.manifest
    if manifest_path is None and args.zip:
        manifest_path = args.zip.parent / "manifest.json"

    if manifest_path:
        write_manifest(
            manifest_path,
            bundle_version=args.bundle_version,
            content_db=args.output,
            scope=scope,
        )

    if args.zip:
        import zipfile

        args.zip.parent.mkdir(parents=True, exist_ok=True)
        with zipfile.ZipFile(args.zip, "w", zipfile.ZIP_DEFLATED) as zf:
            zf.write(args.output, "content.sqlite")
            zf.write(manifest_path, "manifest.json")
        print(f"wrote {args.zip}")

    print(f"wrote {args.output} ({args.output.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
