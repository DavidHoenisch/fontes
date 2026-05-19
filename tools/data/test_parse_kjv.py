"""Tests for kaiserlik KJV parsing."""

from __future__ import annotations

import unittest
from pathlib import Path

from books import BOOKS
from db import REPO_ROOT
from parse_kjv import parse_book_json

CACHE = REPO_ROOT / ".cache" / "fontes" / "sources" / "kjv"
FIXTURE_BOOKS = ("Jhn", "Mar", "Phm", "1Ki")


class ParseKjvTests(unittest.TestCase):
    @unittest.skipUnless(CACHE.exists(), "KJV cache not present")
    def test_parse_known_books(self) -> None:
        for abbrev in FIXTURE_BOOKS:
            path = CACHE / f"{abbrev}.json"
            if not path.is_file():
                self.skipTest(f"missing {path}")
            verses = parse_book_json(path)
            self.assertGreater(len(verses), 0)
            self.assertEqual(verses[0].book.abbrev, abbrev)
            self.assertNotIn("<", verses[0].plain_text)

    def test_parse_john_fixture_cache_or_skip(self) -> None:
        path = CACHE / "Jhn.json"
        if not path.is_file():
            self.skipTest("Jhn.json not cached")
        verses = parse_book_json(path, chapters={1})
        self.assertTrue(any(v.chapter == 1 and v.verse == 1 for v in verses))


if __name__ == "__main__":
    unittest.main()
